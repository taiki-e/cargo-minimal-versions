// Adapted from https://github.com/taiki-e/cargo-hack/blob/v0.5.6/src/restore.rs.

use std::{
    mem,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use fs_err as fs;
use slab::Slab;

use crate::term;

#[derive(Clone)]
pub(crate) struct Manager {
    /// Information on files that need to be restored.
    /// If `needs_restore` is `false`, this is always empty.
    files: Arc<Mutex<Slab<File>>>,
}

impl Manager {
    pub(crate) fn new() -> Self {
        let this = Self { files: Arc::new(Mutex::new(Slab::new())) };

        let cloned = this.clone();
        ctrlc::set_handler(move || {
            cloned.restore(None);
            if term::error() {
                std::process::exit(1)
            }
            std::process::exit(0)
        })
        .unwrap();

        this
    }

    pub(crate) fn push(&self, text: impl Into<String>, path: &Path) -> Handle<'_> {
        let mut files = self.files.lock().unwrap();
        let entry = files.vacant_entry();
        let key = entry.key();
        entry.insert(File { text: text.into(), path: path.to_owned() });

        Handle(Some((self, key)))
    }

    fn restore(&self, key: Option<usize>) {
        let mut files = self.files.lock().unwrap();
        if let Some(key) = key {
            if let Some(file) = files.try_remove(key) {
                file.restore();
            }
        } else if !files.is_empty() {
            for (_, file) in mem::take(&mut *files) {
                file.restore();
            }
        }
    }
}

struct File {
    /// The original text of this file.
    text: String,
    /// Path to this file.
    path: PathBuf,
}

impl File {
    fn restore(self) {
        if term::verbose() {
            info!("restoring {}", self.path.display());
        }
        if let Err(e) = fs::write(&self.path, &self.text) {
            error!("{:#}", e);
        }
    }
}

#[must_use]
pub(crate) struct Handle<'a>(Option<(&'a Manager, usize)>);

impl Drop for Handle<'_> {
    fn drop(&mut self) {
        if let Some((manager, key)) = self.0.take() {
            manager.restore(Some(key));
        }
    }
}
