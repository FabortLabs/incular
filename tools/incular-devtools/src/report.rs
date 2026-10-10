use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

/// Saves a report to the user's Downloads folder (or the OS temporary directory).
pub(crate) fn save(report: &serde_json::Value) -> io::Result<PathBuf> {
    let directory = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(std::path::Path::to_owned))
        .unwrap_or_else(std::env::temp_dir)
        .join("Incular DevTools");
    std::fs::create_dir_all(&directory)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let path = directory.join(format!(
        "diagnostics-{timestamp}-{}.json",
        std::process::id()
    ));
    let mut file = io::BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?,
    );
    serde_json::to_writer_pretty(&mut file, report)?;
    file.flush()?;
    Ok(path)
}
