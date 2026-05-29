use clap::Parser;
use color_eyre::eyre::eyre;
use ratatui::{DefaultTerminal, Frame, widgets::Widget};

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

    ratatui::run(app)?;
    Ok(())
}

fn get_acceptable_default_program() -> Vec<&'static str> {
    DEFAULT_PROGRAMS
        .iter()
        .find(|program| which::which(program[0]).is_ok())
        .copied()
        .unwrap_or(DEFAULT_PROGRAMS.last().unwrap())
        .to_vec()
}

pub struct App {
    max_height: u32,
    output: String,
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        todo!()
    }
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(render)?;
        if crossterm::event::read()?.is_key_press() {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello world", frame.area());
}
