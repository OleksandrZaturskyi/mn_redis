use tokio::sync::mpsc::{self, Sender};
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(1);

    for i in 0..10 {
        tokio::spawn(some_operation(i, tx.clone()));
    }

    drop(tx);

    let _ = rx.recv().await;
}

async fn some_operation(i: u64, _sender: Sender<()>) {
    time::sleep(Duration::from_millis(100 * i)).await;
    println!("Task {i} shutting down")
}
