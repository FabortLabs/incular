//! Build-driver client for development hot reload.
//!
//! `dx serve --hot-patch` starts the application with the devserver address in
//! its environment and publishes one JSON message per compiled patch on a
//! local websocket. This module is the only place that knows that protocol:
//! it extracts the patches addressed to this process and hands them to the UI
//! thread, which owns patch application. Another driver can replace it
//! without touching the runtime.

use incular_runtime::HotPatch;

const DEVSERVER_IP: &str = "DIOXUS_DEVSERVER_IP";
const DEVSERVER_PORT: &str = "DIOXUS_DEVSERVER_PORT";
const BUILD_ID: &str = "DIOXUS_BUILD_ID";

/// The devserver that launched this process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevServer {
    address: std::net::SocketAddr,
    build_id: u64,
}

impl DevServer {
    /// Reads the devserver address published by the build driver. Returns
    /// `None` when the process was started any other way.
    #[must_use]
    pub fn from_environment() -> Option<Self> {
        let read = |name| std::env::var(name).ok();
        Self::from_values(
            read(DEVSERVER_IP).as_deref(),
            read(DEVSERVER_PORT).as_deref(),
            read(BUILD_ID).as_deref(),
        )
    }

    /// Environment-independent form of [`Self::from_environment`]. A missing
    /// or unparseable address yields `None`; a missing build id is build `0`.
    #[must_use]
    pub fn from_values(
        ip: Option<&str>,
        port: Option<&str>,
        build_id: Option<&str>,
    ) -> Option<Self> {
        let address = format!("{}:{}", ip?, port?).parse().ok()?;
        let build_id = build_id.and_then(|id| id.parse().ok()).unwrap_or(0);
        Some(Self { address, build_id })
    }

    /// The build this process belongs to; patches for other builds are ignored.
    #[must_use]
    pub const fn build_id(&self) -> u64 {
        self.build_id
    }

    /// Websocket URL identifying this process to the devserver, which needs
    /// the loaded image's reference address to rebase each patch.
    #[must_use]
    pub fn url(&self, aslr_reference: u64, pid: u32) -> String {
        format!(
            "ws://{}/_dioxus?aslr_reference={aslr_reference}&build_id={}&pid={pid}",
            self.address, self.build_id
        )
    }
}

#[derive(serde::Deserialize)]
struct DevServerMessage {
    #[serde(rename = "HotReload")]
    hot_reload: HotReloadMessage,
}

#[derive(serde::Deserialize)]
struct HotReloadMessage {
    jump_table: Option<HotPatch>,
    for_build_id: Option<u64>,
    for_pid: Option<u32>,
}

/// Extracts the patch one devserver message carries for this process.
///
/// Every other message kind, asset-only reloads, and patches addressed to
/// another process or build yield `None`: applying a patch built for a
/// different image would redirect calls to unrelated addresses.
#[must_use]
pub fn patch_for_process(message: &str, build_id: u64, pid: u32) -> Option<HotPatch> {
    let message = serde_json::from_str::<DevServerMessage>(message)
        .ok()?
        .hot_reload;
    if message.for_pid != Some(pid) || message.for_build_id.is_some_and(|id| id != build_id) {
        return None;
    }
    message.jump_table
}

/// Listens for patches on a background thread for the life of the process,
/// passing each one addressed to this process to `deliver`.
pub(crate) fn connect(
    server: DevServer,
    aslr_reference: u64,
    deliver: impl Fn(HotPatch) + Send + 'static,
) {
    let pid = std::process::id();
    let url = server.url(aslr_reference, pid);
    let listen = move || {
        let mut socket = match tungstenite::connect(url.as_str()) {
            Ok((socket, _response)) => socket,
            Err(error) => {
                eprintln!("Incular hot reload: devserver unavailable at {url}: {error}");
                return;
            }
        };
        while let Ok(message) = socket.read() {
            if let tungstenite::Message::Text(text) = message
                && let Some(patch) = patch_for_process(&text, server.build_id(), pid)
            {
                deliver(patch);
            }
        }
    };
    if let Err(error) = std::thread::Builder::new()
        .name("incular-hot-reload".to_owned())
        .spawn(listen)
    {
        eprintln!("Incular hot reload: listener thread unavailable: {error}");
    }
}
