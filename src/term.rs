use std::io::{stdout, Result};
use std::panic;

use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal::enable_raw_mode};
use ratatui::{backend::Backend, Terminal};

use crate::app::App;

pub struct Term<B: Backend> {
    term: Terminal<B>,
}

impl<B> Term<B>
where
    B: Backend,
{
    pub fn new(term: Terminal<B>) -> Self {
        Self { term }
    }

    pub fn init(&mut self) -> Result<()> {
        execute!(stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;

        let old = panic::take_hook();
        panic::set_hook(Box::new(move |pi| {
            Self::reset().expect("failed to reset terminal");
            old(pi);
        }));

        self.term.clear()?;
        Ok(())
    }

    pub async fn run<'a>(&'a mut self, app: &'a mut App<'a>) -> Result<()> {
        app.run(&mut self.term).await
    }

    pub fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        Ok(())
    }

    fn reset() -> Result<()> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        Ok(())
    }
}
