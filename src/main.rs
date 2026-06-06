use clap::Parser;
use color_eyre::{Result, eyre::eyre};

pub mod grep;
pub mod proc;
pub mod ui;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    grep_program: Option<String>,
}

fn main() -> Result<()> {
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/tmp", "prefix.log");
    tracing_subscriber::fmt().with_writer(file_appender).init();
    color_eyre::install().expect("failed to exceed color_eyre");
    let args = Args::parse();

    let parsed_grep_program =
        grep_from_args(args.grep_program).expect("failed to determine grep-program");

    let spawner = grep::GrepSpawner::new(parsed_grep_program);
    ratatui::run(|terminal| ui::App::new(spawner).run(terminal)).expect("failed to run app");
    Ok(())
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
        .iter()
        .find(|&program| which::which(program[0]).is_ok())
        .copied()
        .unwrap_or(
            default_programs
                .last()
                .expect("must have at least one default"),
        );
    chosen_program.iter().map(|&s| s.to_owned()).collect()
}
