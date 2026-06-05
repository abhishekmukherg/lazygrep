use std::{process::Stdio, sync::Arc};

use color_eyre::{Result, eyre::eyre};
use nix::unistd::Pid;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStdout, Command},
    sync::RwLock,
};

pub(crate) struct GrepSpawner {
    command_line: Vec<String>,
}

impl GrepSpawner {
    pub fn new(command_line: Vec<String>) -> Self {
        Self { command_line }
    }
    pub fn spawn(&self, _query: &str) -> Result<Grepper<Unpaused>> {
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
            command_line,
            query: query.to_string(),
            process: child,
            results,
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
    pub fn pause(self) -> Result<Grepper<Paused>> {
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
    pub fn unpause(self) -> Result<Grepper<Unpaused>> {
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
