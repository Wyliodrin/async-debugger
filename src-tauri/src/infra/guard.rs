use crate::error::Error as TraceError;
use log::{error, info};
use serde::Serialize;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::RwLockWriteGuard;

pub trait DataBaseWrite<D: Serialize + Clone> {
    fn writeable(&mut self) -> &mut D;
}

pub struct WriteableDataBaseGuard<'a, D: Serialize> {
    pub(crate) folder: &'a str,
    pub(crate) title: &'a str,
    pub(crate) elements: RwLockWriteGuard<'a, D>,
}

impl<D: Serialize> Deref for WriteableDataBaseGuard<'_, D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

impl<D: Serialize> DerefMut for WriteableDataBaseGuard<'_, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}

impl<D: Serialize + Clone> DataBaseWrite<D> for Arc<D> {
    fn writeable(&mut self) -> &mut D {
        Arc::make_mut(self)
    }
}

impl<D: Serialize> Drop for WriteableDataBaseGuard<'_, D> {
    fn drop(&mut self) {
        let filename = format!("{}/{}.json", self.folder, self.title);
        info!("Storing {} to {filename}", self.title);
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
