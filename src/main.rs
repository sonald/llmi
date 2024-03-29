use dotenv::dotenv;
use llmi::{app::App, term::Term};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Result};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let mut term = Term::new(Terminal::new(CrosstermBackend::new(stdout()))?);
    term.init()?;

    let mut app = App::new();
    term.run(&mut app).await?;

    term.exit()?;
    Ok(())
}
