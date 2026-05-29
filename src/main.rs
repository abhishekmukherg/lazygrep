use clap::Parser;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LazygrepError {
    #[error("invalid arguments")]
    InvalidArguments,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    grep_program: Option<String>,
}

const DEFAULT_PROGRAMS: &[&[&str]] = &[&["rg"], &["grep", "-R"]];

fn main() -> Result<(), LazygrepError> {
    let args = Args::parse();

    let parsed_grep_program = args
        .grep_program
        .map(|grep_arg| shlex::split(&grep_arg).ok_or(LazygrepError::InvalidArguments))
        .transpose()?
        .unwrap_or_else(|| {
            get_acceptable_default_program()
                .iter()
                .map(|&s| String::from(s))
                .collect()
        });

    println!("Chosen grep: {:?}", parsed_grep_program);
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
