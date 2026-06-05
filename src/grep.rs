use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    process::{Child, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex, RwLock},
    thread,
};

use color_eyre::{Result, eyre::eyre};
use nix::unistd::Pid;

#[derive(Debug)]
pub(crate) struct GrepSpawner {
    command_line: Arc<Vec<String>>,
    paused_greps: Mutex<HashMap<String, Grepper<Paused>>>,
}

impl GrepSpawner {
    pub(crate) fn new(command_line: Vec<String>) -> Self {
        Self {
            command_line: Arc::new(command_line),
            paused_greps: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn spawn(
        &mut self,
        query: &str,
        replace: Option<Grepper<Unpaused>>,
    ) -> Result<Grepper<Unpaused>> {
        let mut paused_greps = self
            .paused_greps
            .lock()
            .or(Err(eyre!("failed to lock paused_greps")))?;

        if let Some(replace) = replace {
            let replaced = replace
                .pause()
                .expect("should always succeed to pause (probably exited)");
            let old_query = &replaced.query;
            paused_greps.insert(old_query.clone(), replaced);
        }
        match paused_greps.remove(query) {
            Some(grepper) => Ok(grepper.unpause().expect("should always succed to unpause")),
            None => Grepper::<Unpaused>::new(self.command_line.clone(), query),
        }
    }
}

#[derive(Debug)]
struct Paused;

#[derive(Debug)]
pub struct Unpaused;

#[derive(Debug)]
pub struct Grepper<State> {
    command_line: Arc<Vec<String>>,
    query: String,
    process: Child,
    pub results: Arc<RwLock<Vec<String>>>,
    _state: std::marker::PhantomData<State>,
}

impl<State> Grepper<State> {
    pub fn new(command_line: Arc<Vec<String>>, query: &str) -> Result<Grepper<Unpaused>> {
        let mut cmd = Command::new(command_line[0].clone());
        command_line.iter().skip(1).for_each(|f| {
            cmd.arg(f);
        });
        cmd.arg(query);
        cmd.stdout(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().ok_or(eyre!("failed to take stdout"))?;
        let results = Arc::new(RwLock::new(Vec::<String>::new()));

        let value = results.clone();
        thread::spawn(move || Grepper::<Unpaused>::reader(stdout, value));

        Ok(Grepper::<Unpaused> {
            command_line,
            query: query.to_string(),
            process: child,
            results,
            _state: std::marker::PhantomData,
        })
    }
    fn reader(stdout: ChildStdout, results: Arc<RwLock<Vec<String>>>) -> Result<()> {
        let reader = BufReader::new(stdout).lines();
        let mut lock = results.write().or(Err(eyre!("failed to lock results")))?;
        for line in reader {
            lock.push(line?);
        }
        Ok(())
    }
}

impl Grepper<Unpaused> {
    fn pause(self) -> Result<Grepper<Paused>> {
        let pid = self.process.id() as i32;
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
        let pid = self.process.id() as i32;
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
