use anyhow::Result;

fn main() -> Result<()> {
    match pdfcat::run() {
        Ok(_) => std::process::exit(0),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1)
        }
    }
}
