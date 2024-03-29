use crate::{llm::Message};
use crossterm::event::Event as CrosstermEvent;
use futures::{FutureExt, StreamExt};
use std::{io::Result, time::Duration};
use tokio::{
    select, spawn,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

#[derive(Debug, Clone)]
pub enum Event {
    TermEvent(CrosstermEvent),
    LLMEventStart,
    LLMEventDelta(Message),
    LLMEventEnd,
    TickEvent,
    Notification(String),
}

#[derive(Debug)]
pub struct EventManager {
    rx: UnboundedReceiver<Event>,
    tx: UnboundedSender<Event>,
    handler: JoinHandle<()>,
}

impl EventManager {
    pub fn new() -> Self {
        let (tx, rx) = unbounded_channel::<Event>();

        let tx2 = tx.clone();
        let handler = spawn(async move {
            let mut term_stream = crossterm::event::EventStream::new();
            let mut tick = tokio::time::interval(Duration::from_millis(250));

            loop {
                let term_event = term_stream.next().fuse();

                select! {
                    _ = tick.tick() => tx2.send(Event::TickEvent).unwrap(),
                    event = term_event => {
                        match event {
                            Some(Ok(event)) => tx2.send(Event::TermEvent(event)).unwrap(),
                            Some(Err(e)) => {
                                println!("Error: {}", e);
                                break;
                            }
                            _ => break,
                        }
                    },

                }
            }
        });

        Self { rx, tx, handler }
    }

    pub fn get_sender(&self) -> UnboundedSender<Event> {
        self.tx.clone()
    }

    pub fn send(&self, event: Event) {
        self.tx.send(event).unwrap();
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx.recv().await.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to receive event from channel",
            )
        })
    }
}
