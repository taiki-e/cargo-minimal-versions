// SPDX-License-Identifier: Apache-2.0 OR MIT

// Adapted from https://github.com/taiki-e/cargo-hack

use std::{
    mem,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

use anyhow::Result;

use crate::{fs, term};

#[derive(Clone)]
pub(crate) struct Manager {
    /// Information on files that need to be restored.
    files: Arc<Mutex<Vec<File>>>,
}

impl Manager {
    pub(crate) fn new() -> Self {
        let this = Self { files: Arc::new(Mutex::new(vec![])) };

        let cloned = this.clone();
        ctrlc::set_handler(move || {
            cloned.restore_all();
            std::process::exit(1)
        })
        .unwrap();

        this
    }

    /// Registers the given path.
    pub(crate) fn register(&self, contents: impl Into<Vec<u8>>, path: impl Into<PathBuf>) {
        let mut files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        files.push(File { contents: contents.into(), path: path.into() });
    }

    pub(crate) fn restore_all(&self) {
        let mut files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        if !files.is_empty() {
            for file in mem::take(&mut *files) {
                if let Err(e) = file.restore() {
                    error!("{e:#}");
                }
            }
        }
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.restore_all();
    }
}

struct File {
    /// The original contents of this file.
    contents: Vec<u8>,
    /// Path to this file.
    path: PathBuf,
}

impl File {
    fn restore(self) -> Result<()> {
        if term::verbose() {
            info!("restoring {}", self.path.display());
        }
        fs::write(&self.path, &self.contents)?;
        Ok(())
    }
}
