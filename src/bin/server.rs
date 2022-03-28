use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6380").await.unwrap();

    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let db = db.clone();

        tokio::spawn(async move {
            handlers::process(socket, db).await;
        });
    }
}

mod handlers {
    use crate::connection::Connection;
    use crate::db::Db;
    use mini_redis::{Command, Frame};
    use tokio::net::TcpStream;

    pub async fn process(socket: TcpStream, db: Db) {
        let mut connection = Connection::new(socket);

        while let Some(frame) = connection.read_frame().await.unwrap() {
            let response = match Command::from_frame(frame).unwrap() {
                Command::Set(cmd) => {
                    db.lock()
                        .unwrap()
                        .insert(cmd.key().to_string(), cmd.value().clone());
                    Frame::Simple("OK".to_string())
                }
                Command::Get(cmd) => match db.lock().unwrap().get(cmd.key()) {
                    Some(v) => Frame::Bulk(v.clone().into()),
                    None => Frame::Null,
                },
                cmd => panic!("unimplemented {cmd:?}"),
            };

            connection.write_frame(&response).await.unwrap();
        }
    }
}

mod db {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use bytes::Bytes;

    pub type Db = Arc<Mutex<HashMap<String, Bytes>>>;

    // pub type ShardedDb = Arc<Vec<Mutex<HashMap<String, Bytes>>>>;

    // fn new_sharded_db(num_shards: usize) -> ShardedDb {
    //     let mut db = Vec::with_capacity(num_shards);
    //     for _ in 0..num_shards {
    //         db.push(Mutex::new(HashMap::new()))
    //     }
    //     Arc::new(db)
    // }
}

mod connection {
    use bytes::{Buf, BytesMut};
    use mini_redis::frame::Error::Incomplete;
    use mini_redis::Frame;
    use std::io::Cursor;
    use std::io::Write;
    use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufWriter};
    use tokio::net::TcpStream;

    pub struct Connection {
        stream: BufWriter<TcpStream>,
        buffer: BytesMut,
    }

    impl Connection {
        pub fn new(stream: TcpStream) -> Self {
            Self {
                stream: BufWriter::new(stream),
                buffer: BytesMut::with_capacity(4096),
            }
        }

        pub async fn read_frame(&mut self) -> mini_redis::Result<Option<Frame>> {
            loop {
                if let Some(frame) = self.parse_frame()? {
                    return Ok(Some(frame));
                }

                if 0 == self.stream.read_buf(&mut self.buffer).await? {
                    if self.buffer.is_empty() {
                        return Ok(None);
                    } else {
                        return Err("connection reset by peer".into());
                    }
                }
            }
        }

        fn parse_frame(&mut self) -> mini_redis::Result<Option<Frame>> {
            let mut buf = Cursor::new(&self.buffer[..]);

            match Frame::check(&mut buf) {
                Ok(_) => {
                    let len = buf.position() as usize;

                    buf.set_position(0);

                    let frame = Frame::parse(&mut buf)?;

                    self.buffer.advance(len);

                    Ok(Some(frame))
                }
                Err(Incomplete) => Ok(None),
                Err(e) => Err(e.into()),
            }
        }

        pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
            match frame {
                Frame::Simple(val) => {
                    self.stream.write_u8(b'+').await?;
                    self.stream.write_all(val.as_bytes()).await?;
                    self.stream.write_all(b"\r\n").await?;
                }
                Frame::Error(val) => {
                    self.stream.write_u8(b'-').await?;
                    self.stream.write_all(val.as_bytes()).await?;
                    self.stream.write_all(b"\r\n").await?
                }
                Frame::Integer(val) => {
                    self.stream.write_u8(b':').await?;
                    self.write_decimal(*val).await?;
                }
                Frame::Null => {
                    self.stream.write_all(b"$-1\r\n").await?;
                }
                Frame::Bulk(val) => {
                    let len = val.len();

                    self.stream.write_u8(b'$').await?;
                    self.write_decimal(len as u64).await?;
                    self.stream.write_all(val).await?;
                    self.stream.write_all(b"\r\n").await?;
                }
                Frame::Array(_val) => unimplemented!(),
            }

            self.stream.flush().await?;

            Ok(())
        }

        async fn write_decimal(&mut self, val: u64) -> io::Result<()> {
            let mut buf = [0u8; 12];
            let mut buf = Cursor::new(&mut buf[..]);
            write!(&mut buf, "{}", val)?;

            let pos = buf.position() as usize;
            self.stream.write_all(&buf.get_ref()[..pos]).await?;
            self.stream.write_all(b"\r\n").await?;

            Ok(())
        }
    }
}
