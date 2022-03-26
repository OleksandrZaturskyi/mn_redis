use command::Command;
use mini_redis::client;
use tokio::sync::{mpsc, oneshot};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:6380").await.unwrap();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get { key, resp } => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                }
                Command::Set { key, value, resp } => {
                    let res = client.set(&key, value).await;
                    let _ = resp.send(res);
                }
            }
        }
    });

    // let mut client = client::connect("127.0.0.1:6380").await.unwrap();

    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = Command::Get {
            key: "hello".into(),
            resp: resp_tx,
        };

        tx.send(cmd).await.unwrap();

        let res = resp_rx.await;
        println!("GOT = {res:?}")
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = Command::Set {
            key: "foo".into(),
            value: "bar".into(),
            resp: resp_tx,
        };

        tx2.send(cmd).await.unwrap();

        let res = resp_rx.await;
        println!("GOT = {res:?}")
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}

mod command {
    use bytes::Bytes;
    use mini_redis::Result;
    use tokio::sync::oneshot::Sender;

    #[derive(Debug)]
    pub enum Command {
        Get {
            key: String,
            resp: Responder<Option<Bytes>>,
        },
        Set {
            key: String,
            value: Bytes,
            resp: Responder<()>,
        },
    }

    type Responder<T> = Sender<Result<T>>;
}
