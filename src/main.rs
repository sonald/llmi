use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use dotenv::dotenv;
use serde_json::json;
use std::env;

use reqwest::blocking::Client as BlockingClient;
use reqwest::header::CONTENT_TYPE;

use ratatui::{
    prelude::*,
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use tui_textarea::TextArea;

use std::{
    io::{stdout, Result, Stdout},
    time::Duration,
};

use llmi::llm::*;

#[derive(Default, Debug)]
struct App<'a> {
    quit: bool,
    last_key: Option<KeyEvent>,
    input: TextArea<'a>,
    messages: Vec<Message>,
    cli: BlockingClient,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn run(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.quit {
            term.draw(|frame| {
                self.render_frame(frame);
            })?;

            self.handle_event()?;
        }

        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame<'_>) {
        let maxh = frame.size().height.min(8).max(1);
        let h = if self.input.lines().len() <= maxh as usize {
            Constraint::Min(self.input.lines().len() as u16 + 2)
        } else {
            Constraint::Length(maxh + 2)
        };
        let [chat_area, inp] =
            Layout::vertical([Constraint::Percentage(100), h]).areas(frame.size());

        frame.render_widget(&*self, chat_area);

        {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan));
            self.input.set_block(block);

            frame.render_widget(self.input.widget(), inp)
        }
    }

    pub fn handle_event(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(33))? {
            self.process_event(event::read()?);
        }

        Ok(())
    }

    fn process_event(&mut self, ev: Event) {
        match ev {
            event::Event::Key(
                kev @ KeyEvent {
                    code,
                    modifiers,
                    kind,
                    ..
                },
            ) => {
                match (code, modifiers, kind) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL, KeyEventKind::Press) => {
                        self.quit = true;
                        return;
                    }
                    (KeyCode::Char('j'), KeyModifiers::CONTROL, _) => {
                        let prompt = self.input.lines().join("\n");
                        self.process_prompt(&prompt);
                        return;
                    }
                    _ => {
                        self.input.input(kev);
                    }
                }

                self.last_key = Some(kev);
            }
            _ => {
                println!("other event: {:?}", ev);
            }
        }
    }

    fn llm_request_blocking(&mut self, prompt: &str) -> Result<Message> {
        let endpoint = env::var("LLM_ENDPOINT").unwrap_or("".to_owned());
        let api_key = env::var("LLM_API_KEY").unwrap_or("".to_owned());
        let model = env::var("LLM_MODEL").unwrap_or("mixtral-8x7b-32768".to_owned());

        let data = json!({
            "model": model,
            "messages": [
               { "role": "user", "content": prompt}
            ]
        });
        let resp = self
            .cli
            .post(endpoint)
            .bearer_auth(api_key)
            .header(CONTENT_TYPE, "application/json")
            .json(&data)
            .send()
            .unwrap();

        let llm_resp = resp
            .json::<LLMResponse>()
            .expect("failed to parse response");

        Ok(llm_resp.extract_message())
    }

    fn process_prompt<S: AsRef<str>>(&mut self, prompt: S) {
        let prompt = prompt.as_ref();
        if prompt.is_empty() {
            return;
        }

        self.messages.push(Message::user(prompt.to_string()));
        let resp = self.llm_request_blocking(prompt).expect("request failed");
        self.messages.push(resp);
        self.clear();
    }

    fn clear(&mut self) {
        self.input.select_all();
        self.input.cut();
    }
}

impl<'a> Widget for &App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = self
            .messages
            .iter()
            .map(|msg| {
                let text = format!("{:?}", msg).cyan();
                Line::from(text)
            })
            .collect::<Vec<_>>();

        let block = Block::default()
            .title_bottom("App")
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center);
        Paragraph::new(lines)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

fn main() -> Result<()> {
    dotenv().ok();

    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
    term.clear()?;

    let mut app = App::new();
    app.run(&mut term)?;

    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
