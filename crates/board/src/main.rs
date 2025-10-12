#[tokio::main]
async fn main() -> anyhow::Result<()> {
    board::run().await
}
