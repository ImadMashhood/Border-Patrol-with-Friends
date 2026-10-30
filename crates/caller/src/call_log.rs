//! Local conversation diagnostics. Only explicitly selected fields belong here;
//! never record authorization headers, provider bodies or private scenario prompts.
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
static WRITE: Mutex<()> = Mutex::new(());
static IDS: AtomicU64 = AtomicU64::new(1);
pub fn next_id() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        IDS.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn record(stage: &str, details: Value) {
    let directory = std::env::var_os("BORDER_CALL_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("output/call-logs"));
    let path = directory.join(format!("calls-{}.jsonl", std::process::id()));
    let Ok(_guard) = WRITE.lock() else {
        return;
    };
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(directory)?;
        if path.metadata().is_ok_and(|m| m.len() > 10 * 1024 * 1024) {
            std::fs::rename(&path, path.with_extension("jsonl.1"))?;
        }
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        let mut line = serde_json::to_vec(
            &json!({"timestamp_ms":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),"pid":std::process::id(),"stage":stage,"details":details}),
        )?;
        line.push(b'\n');
        file.write_all(&line)
    })();
    if let Err(error) = result {
        static WARN: std::sync::Once = std::sync::Once::new();
        WARN.call_once(|| eprintln!("Conversation log could not be written: {error}"));
    }
}
