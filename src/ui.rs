use std::{sync::Mutex, thread, time::Duration};

use color_eyre::{Result, eyre::eyre};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use log::{debug, info};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

use crate::grep::{GrepSpawner, Grepper, Unpaused};

#[derive(Debug)]
pub(crate) struct App {
    grep_spawner: GrepSpawner,
    current_grep: Mutex<Option<Grepper<Unpaused>>>,
    exit: bool,
    query: String,
}

impl App {
    pub fn new(grep_spawner: GrepSpawner) -> Self {
        Self {
            grep_spawner,
            current_grep: Mutex::new(None),
            exit: false,
            query: String::from(""),
        }
    }
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            debug!("Attempting ui cycle");
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?;
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        info!(event:?; "Received event");

        match event {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                kind: _,
                state: _,
            } => {
                self.exit = true;
                Ok(())
            }

            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                kind: _,
                state: _,
            } => {
                if let Some(_) = self.query.pop() {
                    self.reset_grepper()?;
                }
                Ok(())
            }

            KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
                kind: _,
                state: _,
            } => {
                if let Some(char) = code.as_char() {
                    self.query.push(char);
                    self.reset_grepper()?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn reset_grepper(&mut self) -> Result<()> {
        let mut current_grep = self
            .current_grep
            .lock()
            .or(Err(eyre!("failed to get grep lock")))?;
        let new_grep = self
            .grep_spawner
            .spawn(self.query.as_ref(), current_grep.take())?;
        current_grep.replace(new_grep);
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title = Line::from(self.query.clone());
        let block = Block::bordered().title(title);

        let results = {
            let current_grep = self.current_grep.lock().expect("failed to lock grepper");
            current_grep
                .as_ref()
                .map(|grep| {
                    let read_lock = grep.results.read().expect("failed to read lock results");
                    read_lock.iter().map(|f| f.to_string()).collect::<Vec<_>>()
                })
                .or_else(|| Some(vec![String::from("waiting for grep")]))
                .unwrap()
        };

        let results = Text::from(results.into_iter().map(Line::from).collect::<Vec<_>>());

        Paragraph::new(results).block(block).render(area, buf);
    }
}
