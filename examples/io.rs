use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut file = File::create("assets/foo.txt").await?;

    file.write_all(b"some bytes").await?;

    let mut f = File::open("assets/foo.txt").await?;
    let mut buffer = Vec::new();

    let _ = f.read_to_end(&mut buffer).await?;

    println!("the bytes: {buffer:?}");
    Ok(())
}
