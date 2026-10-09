//! Memory and CPU use of a running game process.

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStats {
    /// Resident memory: the Java heap plus everything native (LWJGL, the JVM itself).
    pub memory_bytes: u64,
    /// Share of the whole machine, 0–100 (a game busy on 2 of 8 cores shows 25).
    pub cpu_percent: f32,
}

pub struct ProcessSampler {
    system: System,
    pid: Pid,
    cores: f32,
}

impl ProcessSampler {
    pub fn new(pid: u32) -> Self {
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        let mut sampler = Self {
            system: System::new(),
            pid: Pid::from_u32(pid),
            cores: cores as f32,
        };
        // CPU use is measured between two refreshes: this one is the baseline.
        sampler.refresh();
        sampler
    }

    fn refresh(&mut self) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing().with_memory().with_cpu(),
        );
    }

    /// Usage since the previous call; `None` once the process is gone.
    pub fn sample(&mut self) -> Option<ProcessStats> {
        self.refresh();
        let process = self.system.process(self.pid)?;
        Some(ProcessStats {
            memory_bytes: process.memory(),
            cpu_percent: (process.cpu_usage() / self.cores).clamp(0.0, 100.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_the_current_process() {
        let mut sampler = ProcessSampler::new(std::process::id());
        let stats = sampler.sample().unwrap();
        assert!(stats.memory_bytes > 0);
        assert!((0.0..=100.0).contains(&stats.cpu_percent));
    }

    #[test]
    fn gone_process_has_no_stats() {
        // The test binary itself, which exits right away with `--list`.
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--list")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert!(ProcessSampler::new(pid).sample().is_none());
    }
}
