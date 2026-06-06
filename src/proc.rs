use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, RwLock};
use std::thread;

use color_eyre::Result;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

#[derive(Debug)]
pub struct ManagedProcess {
    command: Option<Command>,
    child: Option<Child>,
    output: Arc<RwLock<Vec<String>>>,
    max_lines: usize,
    reader_thread: Option<thread::JoinHandle<()>>,
}

impl ManagedProcess {
    pub fn new(command: Command, max_lines: usize) -> Self {
        Self {
            command: Some(command),
            child: None,
            output: Arc::new(RwLock::new(Vec::with_capacity(max_lines))),
            max_lines,
            reader_thread: None,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let mut command = self
            .command
            .take()
            .expect("Process already started or command missing");
        command.stdout(Stdio::piped());

        let mut child = command.spawn()?;
        let stdout = child.stdout.take().expect("Failed to open stdout");

        let output = Arc::clone(&self.output);
        let max_lines = self.max_lines;

        let handle = thread::spawn(move || {
            let stdout_reader = BufReader::new(stdout);

            for line in stdout_reader.lines().map_while(Result::ok) {
                let mut output = output.write().expect("lock poisoned");
                if output.len() < max_lines {
                    output.push(line);
                }
            }
        });

        self.child = Some(child);
        self.reader_thread = Some(handle);

        Ok(())
    }

    pub fn send_signal(&self, signal: Signal) -> Result<()> {
        if let Some(child) = &self.child {
            let pid = Pid::from_raw(child.id() as i32);
            kill(pid, signal)?;
        }
        Ok(())
    }

    pub fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    pub fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            let status = child.wait()?;
            if let Some(handle) = self.reader_thread.take() {
                let _ = handle.join();
            }
            Ok(status)
        } else {
            Err(color_eyre::eyre::eyre!("Process not running"))
        }
    }

    pub fn output<B, F>(&self, fun: F) -> Vec<B>
    where
        F: FnMut(&String) -> B,
    {
        self.output.read().unwrap().iter().map(fun).collect()
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn test_managed_process_output() -> Result<()> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("echo line1; echo line2; echo line3");
        let mut proc = ManagedProcess::new(cmd, 2);
        proc.start()?;
        proc.wait()?;
        let output = proc.output(|f| f.to_string());
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], "line1");
        assert_eq!(output[1], "line2");
        Ok(())
    }

    #[test]
    fn test_process_is_drop() -> Result<()> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("sleep 10");
        let mut proc = ManagedProcess::new(cmd, 1);
        proc.start()?;
        let start = Instant::now();
        drop(proc);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(10));
        Ok(())
    }

    #[test]
    fn test_process_is_killed() -> Result<()> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("sleep 10");
        let mut proc = ManagedProcess::new(cmd, 1);
        proc.start()?;
        let start = Instant::now();
        proc.kill()?;
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(10));
        Ok(())
    }

    #[test]
    fn test_managed_process_signal() -> Result<()> {
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let mut proc = ManagedProcess::new(cmd, 10);
        proc.start()?;
        proc.send_signal(Signal::SIGSTOP)?;
        // We can't easily verify it's stopped without more complex logic,
        // but we can at least verify the call succeeds.
        proc.send_signal(Signal::SIGCONT)?;
        proc.kill()?;
        Ok(())
    }
}
