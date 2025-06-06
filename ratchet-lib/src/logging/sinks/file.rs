use crate::logging::{logger::LogSink, LogEvent, LogLevel};
use chrono::Local;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct FileSink {
    path: PathBuf,
    writer: Mutex<BufWriter<File>>,
    min_level: LogLevel,
    max_size: Option<u64>,
    current_size: Mutex<u64>,
}

impl FileSink {
    pub fn new(path: impl AsRef<Path>, min_level: LogLevel) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        let current_size = file.metadata()?.len();
        let writer = BufWriter::new(file);

        Ok(Self {
            path,
            writer: Mutex::new(writer),
            min_level,
            max_size: None,
            current_size: Mutex::new(current_size),
        })
    }

    pub fn with_rotation(mut self, max_size: u64) -> Self {
        self.max_size = Some(max_size);
        self
    }

    fn rotate_if_needed(&self) -> std::io::Result<()> {
        if let Some(max_size) = self.max_size {
            let current_size = *self.current_size.lock().unwrap();
            if current_size >= max_size {
                self.rotate_file()?;
            }
        }
        Ok(())
    }

    fn rotate_file(&self) -> std::io::Result<()> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let rotated_path = self.path.with_file_name(format!(
            "{}.{}",
            self.path.file_stem().unwrap().to_string_lossy(),
            timestamp
        ));

        // Close current file and rename it
        {
            let mut writer = self.writer.lock().unwrap();
            writer.flush()?;
            drop(writer); // Release the lock before renaming
        }

        std::fs::rename(&self.path, &rotated_path)?;

        // Create new file
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let mut writer = self.writer.lock().unwrap();
        *writer = BufWriter::new(new_file);

        let mut current_size = self.current_size.lock().unwrap();
        *current_size = 0;

        Ok(())
    }

    fn write_event(&self, event: &LogEvent) -> std::io::Result<()> {
        let json = serde_json::to_string(event)?;
        let bytes = json.as_bytes();

        let mut writer = self.writer.lock().unwrap();
        writer.write_all(bytes)?;
        writer.write_all(b"\n")?;

        let mut current_size = self.current_size.lock().unwrap();
        *current_size += bytes.len() as u64 + 1; // +1 for newline

        Ok(())
    }
}

impl LogSink for FileSink {
    fn log(&self, event: LogEvent) {
        if event.level < self.min_level {
            return;
        }

        // Check rotation before writing
        if let Err(e) = self.rotate_if_needed() {
            eprintln!("Failed to rotate log file: {}", e);
        }

        if let Err(e) = self.write_event(&event) {
            eprintln!("Failed to write log event to file: {}", e);
        }
    }

    fn flush(&self) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.flush();
        }
    }
}
