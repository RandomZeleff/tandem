//! The game and everything it starts, so stopping it leaves nothing behind.
//!
//! Modpacks ship helpers that outlive a killed game: Crash Assistant starts its own JVM
//! through a short-lived intermediate process, which `taskkill /T` cannot link back to the
//! game, then reports a "crash" and keeps the game's output pipe open. On Windows the game
//! runs in a Job Object (its descendants join it automatically, whatever their parent); on
//! Unix in its own process group.

use tokio::process::Child;

pub struct ProcessGroup {
    #[cfg(windows)]
    job: Option<windows::Job>,
    #[cfg(unix)]
    pgid: Option<u32>,
}

impl ProcessGroup {
    /// Starts tracking a freshly spawned game. Processes it starts from now on are included.
    pub fn track(child: &Child) -> Self {
        #[cfg(windows)]
        {
            let job = child.raw_handle().and_then(|handle| {
                windows::Job::with_process(handle)
                    .inspect_err(|err| tracing::warn!(error = %err, "no job object for the game"))
                    .ok()
            });
            Self { job }
        }
        #[cfg(unix)]
        {
            // `launch::spawn` made the game the leader of its own group.
            Self { pgid: child.id() }
        }
    }

    /// Kills the game and every process it started.
    pub async fn kill(&self, child: &mut Child) {
        #[cfg(windows)]
        if let Some(job) = &self.job {
            if let Err(err) = job.terminate() {
                tracing::warn!(error = %err, "could not terminate the game's job");
            }
        }
        #[cfg(unix)]
        if let Some(pgid) = self.pgid {
            let _ = tokio::process::Command::new("kill")
                .args(["-KILL", &format!("-{pgid}")])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        }
        // Already gone when the group kill worked; covers the case where it could not.
        let _ = child.kill().await;
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::io;
    use std::ptr;

    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
    };

    /// An anonymous job object. Closing it does not kill the game: closing the launcher
    /// while playing must leave the game running.
    pub struct Job(*mut c_void);

    // A job handle is a kernel object handle, usable from any thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn with_process(process: *mut c_void) -> io::Result<Self> {
            // SAFETY: null attributes and name create a fresh anonymous job; the handle is
            // owned by `Job` and closed on drop.
            let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = Job(handle);
            // SAFETY: both handles are valid; `process` belongs to a live child we own.
            if unsafe { AssignProcessToJobObject(job.0, process) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(job)
        }

        pub fn terminate(&self) -> io::Result<()> {
            // SAFETY: `self.0` is a valid job handle for the lifetime of `self`.
            if unsafe { TerminateJobObject(self.0, 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: the handle was created by `CreateJobObjectW` and is closed once.
            unsafe { CloseHandle(self.0) };
        }
    }
}
