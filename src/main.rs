use std::{process::Stdio, sync::Arc};

use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nix::unistd::Pid;
use ratatui::{
    DefaultTerminal, Frame,
    prelude::{Buffer, Rect},
    style::Stylize,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{Child, ChildStdout, Command},
    sync::RwLock,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    grep_program: Option<String>,
}

#[tokio::main]
async fn main() {
    color_eyre::install().expect("failed to exceed color_eyre");
    let args = Args::parse();

    let parsed_grep_program =
        grep_from_args(args.grep_program).expect("failed to determine grep-program");

    println!("parsed grep: {:?}", parsed_grep_program);

    let spawner = GrepSpawner::new(parsed_grep_program);

    // ratatui::run(|terminal| App::default().run(terminal))
}

fn grep_from_args(arg: Option<String>) -> Result<Vec<String>> {
    match arg {
        Some(v) => shlex::split(v.as_str()).ok_or(eyre!("invalid --grep-argument passed")),
        None => Ok(get_acceptable_default_program()),
    }
}

fn get_acceptable_default_program() -> Vec<String> {
    let default_programs = &[&vec!["rg"], &vec!["grep", "-R"]];

    let chosen_program = default_programs
        .into_iter()
        .find(|&program| which::which(program[0]).is_ok())
        .copied()
        .unwrap_or(
            default_programs
                .last()
                .expect("must have at least one default"),
        );
    chosen_program.into_iter().map(|&s| s.to_owned()).collect()
}

#[derive(Debug, Default)]
pub struct App {
    max_height: u32,
    output: String,
    exit: bool,
    query: String,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
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
        match event {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                kind: _,
                state: _,
            } => {
                self.exit = true;
            }
            _ => {}
        }
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

struct GrepSpawner {
    command_line: Vec<String>,
}

impl GrepSpawner {
    fn new(command_line: Vec<String>) -> Self {
        Self {
            command_line: command_line,
        }
    }
    fn spawn(&self, query: &str) -> Result<Grepper<Unpaused>> {
        todo!()
    }
}

struct Paused;
struct Unpaused;

struct Grepper<State> {
    command_line: Vec<String>,
    query: String,
    process: Child,
    results: Arc<RwLock<Vec<String>>>,
    _state: std::marker::PhantomData<State>,
}

impl<State> Grepper<State> {
    pub fn new(command_line: Vec<String>, query: &str) -> Result<Grepper<Unpaused>> {
        let mut cmd = Command::new(command_line[0].clone());
        command_line.iter().skip(1).for_each(|f| {
            cmd.arg(f);
        });
        cmd.arg(query);
        cmd.stdout(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().ok_or(eyre!("failed to take stdout"))?;
        let results = Arc::new(RwLock::new(Vec::<String>::new()));
        tokio::spawn(Grepper::<Unpaused>::reader(stdout, results.clone()));

        Ok(Grepper::<Unpaused> {
            command_line: command_line,
            query: query.to_string(),
            process: child,
            results: results,
            _state: std::marker::PhantomData,
        })
    }
    async fn reader(stdout: ChildStdout, results: Arc<RwLock<Vec<String>>>) -> Result<()> {
        let mut reader = BufReader::new(stdout).lines();
        while let Some(line) = reader.next_line().await? {
            let mut lock = results.write().await;
            lock.push(line);
        }
        Ok(())
    }
}

impl Grepper<Unpaused> {
    pub fn pause(mut self) -> Result<Grepper<Paused>> {
        let pid = self.process.id().ok_or(eyre!("missing process"))? as i32;
        nix::sys::signal::kill(Pid::from_raw(pid), nix::sys::signal::Signal::SIGSTOP)?;
        Ok(Grepper::<Paused> {
            command_line: self.command_line,
            query: self.query,
            process: self.process,
            results: self.results,
            _state: std::marker::PhantomData,
        })
    }
}

impl Grepper<Paused> {
    pub fn unpause(mut self) -> Result<Grepper<Unpaused>> {
        let pid = self.process.id().ok_or(eyre!("missing process"))? as i32;
        nix::sys::signal::kill(Pid::from_raw(pid), nix::sys::signal::Signal::SIGCONT)?;
        Ok(Grepper::<Unpaused> {
            command_line: self.command_line,
            query: self.query,
            process: self.process,
            results: self.results,
            _state: std::marker::PhantomData,
        })
    }
}
