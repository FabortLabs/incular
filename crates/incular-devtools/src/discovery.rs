//! Per-user DevTools session discovery.
//!
//! A live target owns a registration lease. Dropping an old lease removes the
//! record only when the file still names that exact session, so port reuse
//! cannot let an old target delete a newer target's discovery entry.

use incular_devtools_protocol::DiscoveryRecord;
use std::{
    io,
    io::Write,
    path::{Path, PathBuf},
};

const MAX_DISCOVERY_RECORD_BYTES: u64 = 64 * 1024;
const MAX_DISCOVERY_RECORDS: usize = 128;

fn sessions_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "incular", "incular-devtools")
        .map(|dirs| dirs.data_dir().join("sessions"))
}

fn session_path(port: u16) -> Option<PathBuf> {
    sessions_dir().map(|dir| dir.join(format!("session-{port}.json")))
}

/// Data used to publish one target in the per-user discovery directory.
#[derive(Clone)]
pub struct DiscoveryFileEntry {
    /// Operating-system process identifier of the target.
    pub pid: u32,
    /// Human-readable application name presented by DevTools.
    pub app_name: String,
    /// Loopback WebSocket port selected by the target.
    pub port: u16,
    /// Random hexadecimal token required by the authenticated Hello exchange.
    pub auth_token: String,
    /// Unix timestamp in milliseconds used to order candidate sessions.
    pub started_unix_ms: u64,
    /// Wire-protocol version advertised by this target.
    pub protocol_version: u32,
}

/// Ownership lease for one exact discovery record.
pub struct DiscoveryRegistration {
    path: PathBuf,
    identity: DiscoveryRecord,
}

impl DiscoveryRegistration {
    /// Removes this registration if it still belongs to this target.
    pub fn remove(&self) {
        remove_matching_record(&self.path, &self.identity);
    }
}

impl Drop for DiscoveryRegistration {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Writes this target's discovery record using a temporary file and returns
/// ownership of the exact record.
pub fn register_session(entry: DiscoveryFileEntry) -> io::Result<DiscoveryRegistration> {
    let path = session_path(entry.port)
        .ok_or_else(|| io::Error::other("DevTools data directory is unavailable"))?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("invalid DevTools discovery path"))?;
    std::fs::create_dir_all(parent)?;

    let record = DiscoveryRecord {
        pid: entry.pid,
        app_name: entry.app_name,
        port: entry.port,
        auth_token: entry.auth_token,
        started_unix_ms: entry.started_unix_ms,
        protocol_version: entry.protocol_version,
    };
    let json = serde_json::to_vec(&record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if json.len() as u64 > MAX_DISCOVERY_RECORD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DevTools discovery record exceeds the supported size",
        ));
    }

    // tempfile creates an unpredictable, exclusive file with mode 0600 on Unix
    // before any credentials are written. On Windows it inherits the per-user
    // data directory's ACL. Never follow a predictable temporary-file symlink.
    // Keeping the file in this directory allows an atomic rename on persist.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&json)?;
    temporary.flush()?;
    temporary.persist(&path).map_err(|error| error.error)?;

    Ok(DiscoveryRegistration {
        path,
        identity: record,
    })
}

fn remove_matching_record(path: &Path, identity: &DiscoveryRecord) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if metadata.len() > MAX_DISCOVERY_RECORD_BYTES {
        return;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let Ok(current) = serde_json::from_slice::<DiscoveryRecord>(&bytes) else {
        return;
    };
    if current.pid == identity.pid
        && current.port == identity.port
        && current.started_unix_ms == identity.started_unix_ms
        && current.auth_token == identity.auth_token
    {
        let _ = std::fs::remove_file(path);
    }
}

/// Lists readable discovery records. Records whose process is provably gone
/// are skipped and best-effort deleted; where liveness cannot be determined the
/// record is kept.
pub fn list_sessions() -> Vec<(DiscoveryRecord, PathBuf)> {
    let mut out = Vec::new();
    let Some(dir) = sessions_dir() else {
        return out;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten().take(MAX_DISCOVERY_RECORDS) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.len() > MAX_DISCOVERY_RECORD_BYTES {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<DiscoveryRecord>(&bytes) else {
            continue;
        };
        match process_liveness(record.pid) {
            Some(true) | None => out.push((record, path)),
            Some(false) => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    out.sort_by_key(|(record, _)| record.started_unix_ms);
    out
}

#[cfg(target_os = "linux")]
fn process_liveness(pid: u32) -> Option<bool> {
    Some(std::path::Path::new(&format!("/proc/{pid}")).exists())
}

/// Windows: opens the process with the least privilege that can query its
/// exit status. A missing pid is dead; access denial still proves it exists.
#[cfg(windows)]
fn process_liveness(pid: u32) -> Option<bool> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, GetLastError};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    const STILL_ACTIVE: u32 = 259;
    // SAFETY: OpenProcess has no preconditions; the returned handle is closed
    // below on every path that obtained one.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return if GetLastError() == ERROR_ACCESS_DENIED {
                Some(true)
            } else {
                // ERROR_INVALID_PARAMETER: no process has this id.
                Some(false)
            };
        }
        let mut code = 0u32;
        let known = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        known.then_some(code == STILL_ACTIVE)
    }
}

/// Other platforms have no dependency-free liveness probe; treat as alive.
#[cfg(not(any(target_os = "linux", windows)))]
fn process_liveness(_pid: u32) -> Option<bool> {
    None
}
