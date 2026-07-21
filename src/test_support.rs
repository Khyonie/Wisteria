use std::{
    env, fs,
    path::{Path, PathBuf},
    panic::{self, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

static CWD_LOCK: Mutex<()> = Mutex::new(());
static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(prefix: &str) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("wisteria-{prefix}-{}-{id}", std::process::id()));

        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn with_current_dir<T>(path: &Path, action: impl FnOnce() -> T) -> T {
    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(path).unwrap();

    let result = panic::catch_unwind(AssertUnwindSafe(action));

    env::set_current_dir(original_dir).unwrap();

    match result {
        Ok(value) => value,
        Err(payload) => panic::resume_unwind(payload),
    }
}
