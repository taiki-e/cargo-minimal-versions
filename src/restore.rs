// SPDX-License-Identifier: Apache-2.0 OR MIT

// Adapted from https://github.com/taiki-e/cargo-hack

use std::{
    mem,
    path::PathBuf,
    sync::{Arc, Mutex},
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
        let this = Self { files: Arc::new(Mutex::new(Vec::new())) };

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
    pub(crate) fn register(&self, text: Vec<u8>, path: impl Into<PathBuf>) {
        let mut files = self.files.lock().unwrap();
        files.push(File { text: Some(text), path: path.into() });
    }

    /// Registers the given path to be removed on exit.
    pub(crate) fn register_remove(&self, path: impl Into<PathBuf>) {
        let mut files = self.files.lock().unwrap();
        files.push(File { text: None, path: path.into() });
    }

    pub(crate) fn restore_all(&self) {
        let mut files = self.files.lock().unwrap();
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
    /// The original text of this file. None to remove.
    text: Option<Vec<u8>>,
    /// Path to this file.
    path: PathBuf,
}

impl File {
    fn restore(self) -> Result<()> {
        if term::verbose() {
            info!("restoring {}", self.path.display());
        }
        if let Some(text) = &self.text {
            fs::write(&self.path, text)?;
        } else {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}
