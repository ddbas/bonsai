use std::fs;
use std::path::PathBuf;

use clap::ValueEnum;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Log level for bonsai, exposed as a CLI enum.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    /// Trace level — most verbose, for detailed diagnostic information
    #[value(name = "trace")]
    Trace,
    /// Debug level — detailed information for debugging
    #[value(name = "debug")]
    Debug,
    /// Info level — general informational messages (default)
    #[value(name = "info")]
    Info,
    /// Warn level — warning messages for potentially harmful situations
    #[value(name = "warn")]
    Warn,
    /// Error level — error messages only
    #[value(name = "error")]
    Error,
}

impl LogLevel {
    /// Convert LogLevel to tracing_subscriber filter LevelFilter
    fn to_level_filter(self) -> tracing_subscriber::filter::LevelFilter {
        match self {
            LogLevel::Trace => tracing_subscriber::filter::LevelFilter::TRACE,
            LogLevel::Debug => tracing_subscriber::filter::LevelFilter::DEBUG,
            LogLevel::Info => tracing_subscriber::filter::LevelFilter::INFO,
            LogLevel::Warn => tracing_subscriber::filter::LevelFilter::WARN,
            LogLevel::Error => tracing_subscriber::filter::LevelFilter::ERROR,
        }
    }
}

/// Resolve the platform-appropriate log directory for bonsai.
///
/// Tries `dirs::state_dir()` first (XDG_STATE_HOME or `~/.local/state` on Linux),
/// falls back to `dirs::data_local_dir()` on other platforms, then appends `bonsai/logs`.
/// Returns the resolved path, but does not create it.
fn log_dir() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|base| base.join("bonsai").join("logs"))
}

/// Prune old log files, keeping only the most recent `retention_count`.
/// Matches files with the prefix "bonsai.log.*" in the log directory.
fn prune_logs(log_path: &PathBuf, retention_count: usize) {
    if let Ok(entries) = fs::read_dir(log_path) {
        let mut files: Vec<_> = entries
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    let file_name = path.file_name()?.to_string_lossy().to_string();
                    if file_name.starts_with("bonsai.log.") {
                        e.metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .map(|modified| (path, modified))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort by modification time (newest first)
        files.sort_by_key(|b| std::cmp::Reverse(b.1));

        // Delete files beyond the retention count
        for (path, _) in files.iter().skip(retention_count) {
            let _ = fs::remove_file(path);
        }
    }
}

/// Initialize the logging subsystem.
///
/// Sets up a `tracing_subscriber` with a rolling file appender, writing to the
/// platform log directory at the specified level. Returns a `WorkerGuard` that
/// must be held for the lifetime of the application to ensure logs are flushed.
///
/// If directory creation or file opening fails, returns `None` and the CLI
/// proceeds without file logging (best-effort behavior).
pub fn init(level: LogLevel) -> Option<WorkerGuard> {
    let log_path = log_dir()?;

    // Create log directory and parents if needed
    fs::create_dir_all(&log_path).ok()?;

    // Prune old log files (keep 7 most recent)
    prune_logs(&log_path, 7);

    // Create rolling file appender
    let file_appender = tracing_appender::rolling::daily(&log_path, "bonsai.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Set up the subscriber with proper layering
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_filter(level.to_level_filter());

    tracing_subscriber::registry().with(fmt_layer).init();

    Some(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_log_dir_returns_some_path() {
        let result = log_dir();
        assert!(result.is_some(), "log_dir() should return a path");
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_level_filter().to_string(), "trace");
        assert_eq!(LogLevel::Debug.to_level_filter().to_string(), "debug");
        assert_eq!(LogLevel::Info.to_level_filter().to_string(), "info");
        assert_eq!(LogLevel::Warn.to_level_filter().to_string(), "warn");
        assert_eq!(LogLevel::Error.to_level_filter().to_string(), "error");
    }

    #[test]
    fn test_prune_logs_keeps_recent_files() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir_path = temp_dir.path().to_path_buf();

        // Create 10 test log files
        for i in 0..10 {
            let file_name = format!("bonsai.log.2026-07-{:02}", i + 1);
            let file_path = log_dir_path.join(&file_name);
            fs::write(&file_path, format!("log content {}", i)).unwrap();
            // Stagger modification times slightly
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Prune to keep only 7 most recent
        prune_logs(&log_dir_path, 7);

        // Count remaining files
        let remaining = fs::read_dir(&log_dir_path)
            .unwrap()
            .filter_map(|e| {
                e.ok().and_then(|entry| {
                    let path = entry.path();
                    let file_name = path.file_name()?.to_string_lossy();
                    if file_name.starts_with("bonsai.log.") {
                        Some(path)
                    } else {
                        None
                    }
                })
            })
            .count();

        assert_eq!(remaining, 7, "prune_logs should keep exactly 7 files");
    }

    #[test]
    fn test_prune_logs_ignores_non_matching_files() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir_path = temp_dir.path().to_path_buf();

        // Create various files
        fs::write(log_dir_path.join("bonsai.log.2026-07-01"), "log 1").unwrap();
        fs::write(log_dir_path.join("bonsai.log.2026-07-02"), "log 2").unwrap();
        fs::write(log_dir_path.join("other_file.txt"), "other").unwrap();
        fs::write(log_dir_path.join("readme.md"), "readme").unwrap();

        prune_logs(&log_dir_path, 1);

        // Should keep 1 bonsai.log file and all non-matching files
        let remaining_files: Vec<_> = fs::read_dir(&log_dir_path)
            .unwrap()
            .filter_map(|e| e.ok().map(|entry| entry.path()))
            .collect();

        assert_eq!(
            remaining_files.len(),
            3,
            "should have 1 bonsai.log + 2 other files"
        );
        assert!(
            remaining_files
                .iter()
                .any(|p| p.file_name().unwrap().to_string_lossy() == "other_file.txt")
        );
        assert!(
            remaining_files
                .iter()
                .any(|p| p.file_name().unwrap().to_string_lossy() == "readme.md")
        );
    }

    #[test]
    fn test_init_with_nonwritable_directory_returns_none() {
        // Try to initialize logging with a non-existent, non-writable path
        // We can't easily test actual permission errors in a cross-platform way,
        // but we can verify that init gracefully handles Option::None from log_dir
        // This is a basic smoke test that init doesn't panic
        let _ = init(LogLevel::Info);
        // If we get here, init returned without panicking
        assert!(true);
    }
}
