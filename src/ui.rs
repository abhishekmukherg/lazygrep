use std::{sync::Mutex, thread, time::Duration};

use color_eyre::{Result, eyre::eyre};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Rect, Size},
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use ratatui_textarea::{Input, Key, TextArea};

use crate::grep::{GrepSpawner, Grepper};

#[derive(Debug)]
pub(crate) struct App<'a> {
    grep_spawner: GrepSpawner,
    current_grep: Mutex<Option<Grepper>>,
    exit: bool,
    textarea: TextArea<'a>,
}

impl App<'_> {
    pub fn new(grep_spawner: GrepSpawner) -> Self {
        Self {
            grep_spawner,
            current_grep: Mutex::new(None),
            exit: false,
            textarea: TextArea::default(),
        }
    }
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event.into())?;
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, event: Input) -> Result<()> {
        match event {
            Input {
                key: Key::Char('c'),
                ctrl: true,
                alt: false,
                ..
            } => {
                self.exit = true;
                Ok(())
            }
            Input {
                key: Key::Char('m'),
                ctrl: true,
                alt: false,
                ..
            }
            | Input {
                key: Key::Enter, ..
            } => Ok(()),

            _ => {
                self.textarea.input(event);
                self.reset_grepper()
            }
        }
    }

    fn reset_grepper(&mut self) -> Result<()> {
        let mut current_grep = self
            .current_grep
            .lock()
            .or(Err(eyre!("failed to get grep lock")))?;
        let query = self.textarea.lines().join("\n");
        let new_grep = self
            .grep_spawner
            .spawn(query.as_ref(), current_grep.take())?;
        current_grep.replace(new_grep);
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let (first_line_area, remainder_area) = App::areas(&frame.area());
        frame.render_widget(&self.textarea, first_line_area);
        frame.render_widget(self, remainder_area);
    }

    fn areas(area: &Rect) -> (Rect, Rect) {
        let first_line_size = Size::from((area.width, 1));
        let first_line = Rect::from((area.as_position(), first_line_size));

        let remainder_size = Size::from((area.width, area.height - 1));
        let mut remainder_pos = area.as_position();
        remainder_pos.y += 1;
        let remainder = Rect::from((remainder_pos, remainder_size));
        (first_line, remainder)
    }
}

impl Widget for &App<'_> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::bordered();

        let results = {
            let current_grep = self.current_grep.lock().expect("failed to lock grepper");
            current_grep
                .as_ref()
                .map(|grep| grep.output(|f| Line::from(f.to_string())))
                .or_else(|| Some(vec![Line::from("waiting for grep")]))
                .unwrap()
        };

        let results = Text::from(results);

        Paragraph::new(results).block(block).render(area, buf);
    }
}
