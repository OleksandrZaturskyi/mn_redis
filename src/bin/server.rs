use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

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
    use crate::db::Db;
    use mini_redis::{Command, Connection, Frame};
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
