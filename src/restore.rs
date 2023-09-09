// Adapted from https://github.com/taiki-e/cargo-hack

use std::{
    mem,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use fs_err as fs;
use slab::Slab;

use crate::term;

#[derive(Clone)]
pub(crate) struct Manager {
    /// Information on files that need to be restored.
    files: Arc<Mutex<Slab<File>>>,
}

impl Manager {
    pub(crate) fn new() -> Self {
        let this = Self { files: Arc::new(Mutex::new(Slab::new())) };

        let cloned = this.clone();
        ctrlc::set_handler(move || {
            cloned.restore_all();
            if term::error() {
                std::process::exit(1)
            }
            std::process::exit(0)
        })
        .unwrap();

        this
    }

    /// Registers the given path.
    pub(crate) fn register(&self, text: impl Into<String>, path: impl Into<PathBuf>) -> Handle<'_> {
        let mut files = self.files.lock().unwrap();
        let entry = files.vacant_entry();
        let key = entry.key();
        entry.insert(File { text: text.into(), path: path.into() });

        Handle(Some((self, key)))
    }

    fn restore(&self, key: usize) -> Result<()> {
        let mut files = self.files.lock().unwrap();
        if let Some(file) = files.try_remove(key) {
            file.restore()?;
        }
        Ok(())
    }

    fn restore_all(&self) {
        let mut files = self.files.lock().unwrap();
        if !files.is_empty() {
            for (_, file) in mem::take(&mut *files) {
                if let Err(e) = file.restore() {
                    error!("{e:#}");
                }
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
    fn restore(self) -> Result<()> {
        if term::verbose() {
            info!("restoring {}", self.path.display());
        }
        fs::write(&self.path, &self.text)?;
        Ok(())
    }
}

#[must_use]
pub(crate) struct Handle<'a>(Option<(&'a Manager, usize)>);

impl Handle<'_> {
    pub(crate) fn close(&mut self) -> Result<()> {
        if let Some((manager, key)) = self.0.take() {
            manager.restore(key)?;
        }
        Ok(())
    }
}

impl Drop for Handle<'_> {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            error!("{e:#}");
        }
    }
}
