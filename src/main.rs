use dotenv::dotenv;
use llmi::{app::App, term::Term};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io::{stdout, Result}};

// Add mod and use for Config
mod config;
use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Load config
    let app_config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            // Using std::io::Error to fit the Result<()> return type of main
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        }
    };

    let mut term = Term::new(Terminal::new(CrosstermBackend::new(stdout()))?);
    term.init()?;

    // Pass config to App::new()
    let mut app = App::new(app_config);
    term.run(&mut app).await?;

    term.exit()?;
    Ok(())
}
