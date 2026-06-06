use std::{
    collections::HashMap,
    process::Command,
    sync::{Arc, Mutex},
};
use tracing::{Level, event};

use crate::proc::ManagedProcess;
use color_eyre::{Result, eyre::eyre};
use nix::sys::signal::Signal;

#[derive(Debug)]
pub(crate) struct GrepSpawner {
    command_line: Arc<Vec<String>>,
    paused_greps: Mutex<HashMap<String, Grepper>>,
}

impl GrepSpawner {
    pub(crate) fn new(command_line: Vec<String>) -> Self {
        Self {
            command_line: Arc::new(command_line),
            paused_greps: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn spawn(&mut self, query: &str, replace: Option<Grepper>) -> Result<Grepper> {
        let mut paused_greps = self
            .paused_greps
            .lock()
            .or(Err(eyre!("failed to lock paused_greps")))?;

        if let Some(replace) = replace {
            let replaced = replace.pause().expect("should always succeed to pause");
            let old_query = &replaced.query;
            paused_greps.insert(old_query.clone(), replaced);
        }
        match paused_greps.remove(query) {
            Some(grepper) => Ok(grepper.unpause().expect("should always succed to unpause")),
            None => {
                event!(Level::INFO, "spawned grepper {}", query);
                Grepper::new(self.command_line.clone(), query)
            }
        }
    }
}

impl Drop for GrepSpawner {
    fn drop(&mut self) {
        let mut greps = self.paused_greps.lock().expect("must get a lock to drop");
        let greps = greps.drain();
        greps
            .into_iter()
            .map(|(_, mut v)| v.process.kill().or(Err(eyre!("failed to kill process"))))
            .collect::<Result<Vec<_>>>()
            .expect("failed to kill process");
    }
}

#[derive(Debug)]
pub struct Grepper {
    query: String,
    process: ManagedProcess,
}

impl Grepper {
    fn new(command_line: Arc<Vec<String>>, query: &str) -> Result<Self> {
        let mut cmd = Command::new(command_line[0].clone());
        command_line.iter().skip(1).for_each(|f| {
            cmd.arg(f);
        });
        cmd.arg(query);
        let mut mp = ManagedProcess::new(cmd, 10);
        mp.start()?;
        Ok(Self {
            query: query.to_string(),
            process: mp,
        })
    }

    fn pause(self) -> Result<Grepper> {
        self.process
            .send_signal(Signal::SIGSTOP)
            .or(Err(eyre!("failed to pause process")))?;
        Ok(self)
    }

    fn unpause(self) -> Result<Grepper> {
        self.process
            .send_signal(Signal::SIGCONT)
            .or(Err(eyre!("failed to pause process")))?;
        Ok(self)
    }

    pub fn output<B, F>(&self, fun: F) -> Vec<B>
    where
        F: FnMut(&String) -> B,
    {
        self.process.output(fun)
    }
}
