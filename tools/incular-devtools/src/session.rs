use incular_devtools_protocol::DiscoveryRecord;
use std::path::Path;

const MAX_DISCOVERY_RECORDS: usize = 128;
const MAX_DISCOVERY_RECORD_BYTES: u64 = 64 * 1024;
const MAX_DISCOVERY_WARNINGS: usize = 32;

#[derive(Clone, Debug, Default)]
pub struct DiscoveryReport {
    pub sessions: Vec<DiscoveryRecord>,
    pub warnings: Vec<String>,
}

impl DiscoveryReport {
    fn warn(&mut self, warning: impl Into<String>) {
        if self.warnings.len() < MAX_DISCOVERY_WARNINGS {
            self.warnings.push(warning.into());
        }
    }
}

pub(crate) fn sessions_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("dev", "incular", "incular-devtools")
        .map(|dirs| dirs.data_dir().join("sessions"))
}

pub fn scan_sessions() -> DiscoveryReport {
    let mut report = DiscoveryReport::default();
    let Some(dir) = sessions_dir() else {
        report.warn("DevTools data directory is unavailable");
        return report;
    };
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            report.warn("DevTools session directory does not exist yet");
            return report;
        }
        Err(error) => {
            report.warn(format!(
                "unable to read DevTools session directory: {error}"
            ));
            return report;
        }
    };
    let mut considered = 0_usize;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.warn(format!(
                    "unable to read a DevTools discovery entry: {error}"
                ));
                continue;
            }
        };
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        if considered == MAX_DISCOVERY_RECORDS {
            report.warn(format!(
                "DevTools discovery scan stopped after {MAX_DISCOVERY_RECORDS} records"
            ));
            break;
        }
        considered += 1;
        let display_name = discovery_name(&path);
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                report.warn(format!("unable to inspect {display_name}: {error}"));
                continue;
            }
        };
        if metadata.len() > MAX_DISCOVERY_RECORD_BYTES {
            report.warn(format!(
                "ignored oversized discovery record {display_name} ({} bytes)",
                metadata.len()
            ));
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                report.warn(format!("unable to read {display_name}: {error}"));
                continue;
            }
        };
        let record = match serde_json::from_slice::<DiscoveryRecord>(&bytes) {
            Ok(record) => record,
            Err(_) => {
                report.warn(format!("ignored malformed discovery record {display_name}"));
                continue;
            }
        };
        if record.port == 0
            || record.auth_token.len() != 32
            || !record
                .auth_token
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            report.warn(format!("ignored invalid discovery record {display_name}"));
            continue;
        }
        match record_alive(&record) {
            Some(true) | None => report.sessions.push(record),
            Some(false) => {
                if let Err(error) = std::fs::remove_file(path) {
                    report.warn(format!(
                        "stale discovery record {display_name} could not be removed: {error}"
                    ));
                }
            }
        }
    }
    report.sessions.sort_by_key(|record| record.started_unix_ms);
    report
}

/// Best-effort liveness check for a discovery record.
///
/// Linux asks `/proc` and Windows asks the process handle for its exit code.
/// macOS has no portable process-existence API in the standard library, so
/// there the record's loopback port is probed with a short connect: a dead
/// process refuses immediately, so its leftover record is treated as stale and
/// removed. The probe sends no bytes, so it never reaches the target's
/// authenticated session.
fn record_alive(record: &DiscoveryRecord) -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        Some(std::path::Path::new(&format!("/proc/{}", record.pid)).exists())
    }
    #[cfg(windows)]
    {
        windows_process_alive(record.pid)
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        use std::net::{Ipv4Addr, SocketAddr, TcpStream};
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, record.port));
        match TcpStream::connect_timeout(&address, std::time::Duration::from_millis(250)) {
            Ok(_) => Some(true),
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => None,
            Err(_) => Some(false),
        }
    }
}

/// Opens the process with the least privilege that can query its exit status.
/// A missing pid is dead; access denial still proves the process exists.
#[cfg(windows)]
fn windows_process_alive(pid: u32) -> Option<bool> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, GetLastError};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    const STILL_ACTIVE: u32 = 259;
    // SAFETY: OpenProcess has no preconditions; the handle it returns is
    // closed below on the only path that obtained one.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return Some(GetLastError() == ERROR_ACCESS_DENIED);
        }
        let mut code = 0_u32;
        let known = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        known.then_some(code == STILL_ACTIVE)
    }
}

fn discovery_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<non-utf8 discovery record>")
        .to_owned()
}

pub fn requested_target_pid(
    args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
) -> Option<u32> {
    let mut arguments = args.into_iter();
    while let Some(argument) = arguments.next() {
        if argument.as_ref() == "--target-pid"
            && let Some(pid) = arguments
                .next()
                .and_then(|value| value.as_ref().to_str()?.parse().ok())
        {
            return Some(pid);
        }
    }
    None
}

pub fn select_session(
    sessions: &[DiscoveryRecord],
    target_pid: Option<u32>,
) -> Option<DiscoveryRecord> {
    target_pid.map_or_else(
        || sessions.last().cloned(),
        |pid| sessions.iter().find(|record| record.pid == pid).cloned(),
    )
}
