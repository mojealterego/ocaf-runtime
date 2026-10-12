//! Exclusive coordinator ownership on one reliable local filesystem.
//!
//! The operating system releases the file lock when a coordinator process
//! terminates, allowing a warm standby on the SAME HOST to take over the
//! shared SQLite replay, membership and job journals. This does not supply
//! cross-host consensus, storage replication, or network partition fencing.

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

use fs2::FileExt;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LeadershipError {
    #[error("leader lock requires an absolute regular-file path")]
    InvalidPath,
    #[error("leader lock filesystem error")]
    Io,
}

#[derive(Debug)]
pub struct LeaderGuard {
    _lock: File,
}

impl LeaderGuard {
    /// Acquire a kernel file lock or return None if another coordinator holds
    /// it. Never remove/unlink the lock file, even after release: unlinking
    /// allows two distinct inodes to be locked at once.
    pub fn try_acquire(path: &Path) -> Result<Option<Self>, LeadershipError> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(LeadershipError::InvalidPath);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let parent = path.parent().ok_or(LeadershipError::InvalidPath)?;
            let meta = std::fs::metadata(parent).map_err(|_| LeadershipError::Io)?;
            if !meta.is_dir() || meta.permissions().mode() & 0o022 != 0 {
                return Err(LeadershipError::InvalidPath);
            }
        }
        if let Ok(meta) = std::fs::symlink_metadata(path) {
            if !meta.file_type().is_file() {
                return Err(LeadershipError::InvalidPath);
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let lock = options.open(path).map_err(|_| LeadershipError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            lock.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|_| LeadershipError::Io)?;
        }
        match lock.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { _lock: lock })),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(_) => Err(LeadershipError::Io),
        }
    }

    /// A standby promotes ONLY after the original leader releases its
    /// process-bound OS lock (including abnormal process termination).
    /// On filesystem errors it fails closed instead of starting a second
    /// independently writable coordinator.
    pub async fn acquire_standby(
        path: &Path,
        poll_interval: Duration,
    ) -> Result<Self, LeadershipError> {
        if poll_interval.is_zero() || poll_interval > Duration::from_secs(5) {
            return Err(LeadershipError::InvalidPath);
        }
        loop {
            if let Some(guard) = Self::try_acquire(path)? {
                return Ok(guard);
            }
            tokio::time::sleep(poll_interval).await;
        }
    }
}

impl Drop for LeaderGuard {
    fn drop(&mut self) {
        // Releasing explicitly avoids relying on the timing of file
        // descriptor close for same-process active/standby handover.
        // After abnormal process death the kernel still releases the lock.
        let _ = FileExt::unlock(&self._lock);
    }
}
