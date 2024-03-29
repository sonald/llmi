use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::{layout::Constraint, Frame, Terminal};
use tokio::sync::Mutex;
use tui_textarea::TextArea;

use std::io::Result;
use std::sync::Arc;

use crate::event::{Event, EventManager};
use crate::llm::{ChatGPT, Message};
#[derive(Debug)]
pub struct App<'a> {
    event_manager: EventManager,
    quit: bool,
    last_key: Option<KeyEvent>,
    input: TextArea<'a>,
    messages: Vec<Message>, // list of completed messages
    notification: Option<String>,
    current_message: Option<String>, // current message on the fly
    llm: Arc<Mutex<ChatGPT>>,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self {
            event_manager: EventManager::new(),
            quit: false,
            last_key: None,
            input: TextArea::default(),
            messages: Vec::default(),
            notification: None,
            current_message: None,
            llm: Arc::new(Mutex::new(ChatGPT::new())),
        }
    }

    pub async fn run<B: Backend>(&mut self, term: &mut Terminal<B>) -> Result<()> {
        while !self.quit {
            self.render(term)?;

            match self.event_manager.next().await {
                Ok(Event::TermEvent(ev)) => {
                    self.process_event(ev).await;
                }
                Ok(Event::LLMEventDelta(msg)) => {
                    if let Some(ref delta) = msg.content {
                        self.current_message
                            .get_or_insert("".to_string())
                            .push_str(&delta);
                    }
                }
                Ok(Event::LLMEventEnd) => {
                    if let Some(msg) = self.current_message.take() {
                        self.messages.push(Message::assistant(msg));
                    }
                }
                Ok(Event::LLMEventStart) => {
                    self.current_message.take();
                }
                Ok(Event::Notification(msg)) => {
                    self.notification.replace(msg);
                }
                Ok(Event::TickEvent) => {
                    // println!("tick");
                }
                Err(e) => {
                    println!("Error: {}", e);
                    self.quit = true;
                }
            }
        }

        Ok(())
    }

    fn render<B: Backend>(&mut self, term: &mut Terminal<B>) -> Result<()> {
        term.draw(|frame| {
            self.render_frame(frame);
        })?;

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

        // TODO: popup and fade out later
        if let Some(ref notif) = self.notification {
            let lines: Vec<_> = notif
                .chars()
                .collect::<Vec<char>>()
                .chunks(frame.size().width as usize - 2)
                .map(|chunk| Line::from(chunk.iter().collect::<String>()))
                .collect();

            let size = frame.size();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Red));
            frame.render_widget(
                Paragraph::new(lines)
                    .block(block)
                    .style(Style::default().fg(Color::Red)),
                size,
            );
        }
    }

    async fn process_event(&mut self, ev: CrosstermEvent) {
        match ev {
            CrosstermEvent::Key(
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
                        self.process_prompt(&prompt).await;
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

    async fn process_prompt<S: AsRef<str>>(&mut self, prompt: S) {
        let prompt = prompt.as_ref();
        if prompt.is_empty() {
            return;
        }

        let prompt = prompt.to_string();
        self.messages.push(Message::user(prompt.clone()));

        let llm = Arc::clone(&self.llm);
        let tx = self.event_manager.get_sender();
        tokio::spawn(async move {
            let mut llm = llm.lock().await;
            llm.request(&prompt, &tx).await.expect("llm request failed");
        });
        self.clear();
    }

    fn clear(&mut self) {
        self.input.select_all();
        self.input.cut();
    }
}

impl<'a> Widget for &App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut offset = area.y;
        self.messages.iter().for_each(|msg| {
            let sub_area = Rect {
                x: area.x,
                y: offset,
                width: area.width,
                height: msg.len() as u16 + 2,
            };
            offset += sub_area.height;
            msg.render(sub_area, buf)
        });

        if let Some(ref delta) = self.current_message {
            let delta_msg = Message::assistant(delta.clone());
            let sub_area = Rect {
                x: area.x,
                y: offset,
                width: area.width,
                height: delta_msg.len() as u16 + 2,
            };
            delta_msg.render(sub_area, buf);
        }
    }
}
