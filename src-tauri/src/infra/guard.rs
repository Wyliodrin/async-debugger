// infra/guard.rs
//! A guard that auto‐writes a database file on drop. Useful for
//! synchronized, writeable access to in‐memory data.

use crate::error::Error as TraceError;
use log::{error, info};
use serde::Serialize;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::RwLockWriteGuard;

/// Trait implemented by in‐memory databases that support write‐access.
pub trait DataBaseWrite<D: Serialize + Clone> {
    /// Obtain a mutable reference to the underlying data,
    /// potentially cloning if it's shared (e.g. `Arc`).
    #[allow(unused)]
    fn writeable(&mut self) -> &mut D;
}

/// A RAII guard that, when dropped, serializes `elements` back to disk.
pub struct WriteableDataBaseGuard<'a, D: Serialize + Debug> {
    /// Folder path where the JSON file lives.
    pub(crate) folder: &'a str,
    /// Base filename (without extension).
    pub(crate) title: &'a str,
    /// Locked, mutable reference to the data.
    pub(crate) elements: RwLockWriteGuard<'a, D>,
}

impl<D: Serialize + Debug> Deref for WriteableDataBaseGuard<'_, D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

impl<D: Serialize + Debug> DerefMut for WriteableDataBaseGuard<'_, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}

impl<D: Serialize + Clone> DataBaseWrite<D> for Arc<D> {
    /// If the database is behind an `Arc`, this will clone‐on‐write as needed.
    fn writeable(&mut self) -> &mut D {
        Arc::make_mut(self)
    }
}

impl<D: Serialize + Debug> Drop for WriteableDataBaseGuard<'_, D> {
    /// On drop, serialize the data to `{folder}/{title}.json`.
    fn drop(&mut self) {
        let filename = format!("{}/{}.json", self.folder, self.title);
        info!("Storing {} to {filename}", self.title);

        // Pretty‐serialize then write to disk, logging any errors.
        serde_json::to_string_pretty(&*self.elements)
            .map_err(|error| {
                error!("Failed to serialize {filename} ({error})");
                TraceError::Serde(error)
            })
            .map(|json| {
                std::fs::write(&filename, json).map_err(|err| {
                    error!("Failed to write {filename} ({err:?})");
                    TraceError::Anyhow(err.into())
                })
            })
            .ok();

        info!("Dropped {}", self.title);
    }
}
