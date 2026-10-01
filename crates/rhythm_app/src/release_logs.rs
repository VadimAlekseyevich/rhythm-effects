//! Bounded file logging for release builds.
//! Rotation closes the active file before renaming, which is required on Windows.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tracing_subscriber::fmt::MakeWriter;

pub const RELEASE_LOG_FILE: &str = "rhythm-effects.log";
pub const RELEASE_LOG_MAX_BYTES: u64 = 2 * 1024 * 1024;
pub const RELEASE_LOG_ARCHIVES: usize = 4;

#[derive(Debug)]
struct RotatingLogState {
    directory: PathBuf,
    active_path: PathBuf,
    file: Option<File>,
    bytes: u64,
    max_bytes: u64,
    archives: usize,
}

impl RotatingLogState {
    fn open(directory: &Path, max_bytes: u64, archives: usize) -> io::Result<Self> {
        if max_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log rotation size must be nonzero",
            ));
        }
        fs::create_dir_all(directory)?;
        let active_path = directory.join(RELEASE_LOG_FILE);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active_path)?;
        let bytes = file.metadata()?.len();
        let mut state = Self {
            directory: directory.to_path_buf(),
            active_path,
            file: Some(file),
            bytes,
            max_bytes,
            archives,
        };
        if state.bytes >= state.max_bytes {
            state.rotate()?;
        }
        Ok(state)
    }

    fn archive_path(&self, index: usize) -> PathBuf {
        self.directory.join(format!("rhythm-effects.{index}.log"))
    }

    fn rotate(&mut self) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
            file.sync_data()?;
            drop(file);
        }

        if self.archives > 0 {
            let oldest = self.archive_path(self.archives);
            match fs::remove_file(&oldest) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            for index in (1..self.archives).rev() {
                let source = self.archive_path(index);
                let destination = self.archive_path(index + 1);
                match fs::rename(&source, &destination) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            if self.active_path.exists() {
                fs::rename(&self.active_path, self.archive_path(1))?;
            }
        } else {
            match fs::remove_file(&self.active_path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }

        self.file = Some(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.active_path)?,
        );
        self.bytes = 0;
        Ok(())
    }

    fn append(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            if self.bytes >= self.max_bytes {
                self.rotate()?;
            }
            let available = usize::try_from(self.max_bytes - self.bytes).unwrap_or(usize::MAX);
            let take = available.min(bytes.len());
            let (chunk, rest) = bytes.split_at(take);
            self.file
                .as_mut()
                .expect("rotating logger always owns an open file")
                .write_all(chunk)?;
            self.bytes = self
                .bytes
                .saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
            bytes = rest;
        }
        self.file
            .as_mut()
            .expect("rotating logger always owns an open file")
            .flush()
    }
}

/// Cloneable MakeWriter; each tracing event buffers its fragments and commits
/// atomically on flush/drop so lines from concurrent threads do not interleave.
#[derive(Debug, Clone)]
pub struct RotatingLogWriter {
    state: Arc<Mutex<RotatingLogState>>,
}

impl RotatingLogWriter {
    pub fn release(directory: &Path) -> io::Result<Self> {
        Self::new(directory, RELEASE_LOG_MAX_BYTES, RELEASE_LOG_ARCHIVES)
    }

    fn new(directory: &Path, max_bytes: u64, archives: usize) -> io::Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(RotatingLogState::open(
                directory, max_bytes, archives,
            )?)),
        })
    }
}

pub struct RotatingEventWriter {
    state: Arc<Mutex<RotatingLogState>>,
    event: Vec<u8>,
}

impl Write for RotatingEventWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.event.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.event.is_empty() {
            return Ok(());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("release log mutex poisoned"))?;
        state.append(&self.event)?;
        self.event.clear();
        Ok(())
    }
}

impl Drop for RotatingEventWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl<'a> MakeWriter<'a> for RotatingLogWriter {
    type Writer = RotatingEventWriter;

    fn make_writer(&'a self) -> Self::Writer {
        RotatingEventWriter {
            state: Arc::clone(&self.state),
            event: Vec::with_capacity(512),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RELEASE_LOG_FILE, RotatingLogWriter};
    use std::{
        fs,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    use tracing_subscriber::fmt::MakeWriter;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct TestDir(std::path::PathBuf);
    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rhythm-rotating-log-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("test directory");
            Self(path)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn rotation_is_bounded_and_each_file_respects_byte_limit() {
        let root = TestDir::new();
        let writer = RotatingLogWriter::new(&root.0, 64, 2).expect("logger");
        for index in 0..20 {
            let mut event = writer.make_writer();
            writeln!(event, "event-{index:02}-abcdefghijklmnopqrstuvwxyz").expect("event");
        }

        let files: Vec<_> = fs::read_dir(&root.0)
            .expect("list logs")
            .map(|entry| entry.expect("entry").path())
            .collect();
        assert!(files.len() <= 3, "active plus two archives");
        assert!(root.0.join(RELEASE_LOG_FILE).is_file());
        for path in files {
            assert!(
                fs::metadata(&path).expect("metadata").len() <= 64,
                "{} exceeded configured bound",
                path.display()
            );
        }
    }

    #[test]
    fn existing_oversized_active_log_rotates_before_first_new_event() {
        let root = TestDir::new();
        let active = root.0.join(RELEASE_LOG_FILE);
        fs::write(&active, vec![b'x'; 80]).expect("old log");
        let writer = RotatingLogWriter::new(&root.0, 64, 1).expect("logger");
        {
            let mut event = writer.make_writer();
            write!(event, "fresh").expect("event");
        }
        assert_eq!(fs::read(&active).expect("active"), b"fresh");
        assert_eq!(
            fs::metadata(root.0.join("rhythm-effects.1.log"))
                .expect("archive")
                .len(),
            80
        );
    }

    #[test]
    fn zero_rotation_size_is_rejected() {
        let root = TestDir::new();
        assert!(RotatingLogWriter::new(&root.0, 0, 1).is_err());
    }
}
