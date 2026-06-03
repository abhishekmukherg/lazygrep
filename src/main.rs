use std::process::{Child, Command};

use clap::Parser;
use color_eyre::eyre::eyre;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    prelude::{Buffer, Rect},
    style::Stylize,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    grep_program: Option<String>,
}

const DEFAULT_PROGRAMS: &[&[&str]] = &[&["rg"], &["grep", "-R"]];

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let parsed_grep_program = args
        .grep_program
        .map(|grep_arg| shlex::split(&grep_arg).ok_or(eyre!("invalid --grep-program")))
        .transpose()?
        .unwrap_or_else(|| {
            get_acceptable_default_program()
                .iter()
                .map(|&s| String::from(s))
                .collect()
        });

    println!("parsed grep: {:?}", parsed_grep_program);

    ratatui::run(|terminal| App::default().run(terminal))
}

fn get_acceptable_default_program() -> Vec<&'static str> {
    DEFAULT_PROGRAMS
        .iter()
        .find(|program| which::which(program[0]).is_ok())
        .copied()
        .unwrap_or(DEFAULT_PROGRAMS.last().unwrap())
        .to_vec()
}

#[derive(Debug, Default)]
pub struct App {
    max_height: u32,
    output: String,
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?;
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, _: KeyEvent) -> color_eyre::Result<()> {
        self.exit = true;
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
        let title = Line::from("Blah");
        let block = Block::bordered().title(title);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.max_height.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

struct Grepper {
    command_line: Vec<String>,
}

impl Grepper {
    fn spawn(&self, query: &str) -> color_eyre::Result<SpawnedGrepper> {
        todo!()
    }
}

struct SpawnedGrepper {
    command_line: Vec<String>,
    query: String,
    process: Child,
    results: Vec<String>,
}

impl SpawnedGrepper {
    fn new(command_line: Vec<String>, query: &str) -> color_eyre::Result<SpawnedGrepper> {
        let mut cmd = Command::new(command_line[0].clone());
        command_line.iter().skip(1).for_each(|f| {
            cmd.arg(f);
        });
        cmd.arg(query);
        let child = cmd.spawn()?;
        Ok(SpawnedGrepper {
            command_line: command_line,
            query: query.to_string(),
            process: child,
            results: Vec::new(),
        })
    }
}
