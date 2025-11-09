use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    match pdfcat::run().await {
        Ok(_) => std::process::exit(0),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1)
        }
    }
}
