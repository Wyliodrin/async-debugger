use crate::{
    domain::{application::Application, storable::Storable, Task},
    error::Error as TraceError,
    infra::{guard::WriteableDataBaseGuard, storage::Storage},
};
use async_trait::async_trait;
use log::{debug, error};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Representation of all data stored on the disk for persistency
/// Provides read/write mechanisms that assure syncronisation with
/// disk files
#[derive(Default)]
pub(crate) struct Database {
    storage_folder: String,

    // todo: astea trebuie scrise pe disk + incarcate la pornire
    applications: tokio::sync::RwLock<HashMap<Uuid, Arc<Application>>>,
    // toate taskurile curente de la toate aplicatiile
    tasks: tokio::sync::RwLock<HashMap<String, Arc<Task>>>,
}

impl Database {
    /// Is creating a new, fresh database instance withouth
    /// loading from disk
    ///
    /// Is replacing a default implementation, which could not
    /// be added because the Database is used as a dyn Storage
    /// (Self: Sized rule)
    ///
    /// This method should be used if loading failed
    pub(crate) fn new(storage_folder: String) -> Self {
        Self {
            storage_folder,
            applications: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
        }
    }

    /// Is loading the database from the disk
    ///
    /// If file location of any data is not found, a fresh instance will be used.
    ///
    /// # Error
    ///
    /// If failed to load the failed due to unrecoverable errors (eg: failed to serialize)
    /// an error will be returned
    pub(crate) async fn load(storage_folder: String) -> Result<Self, TraceError> {
        // Load all applications
        let applications: HashMap<Uuid, Arc<Application>> =
            match Application::load_all(storage_folder.clone()).await {
                Ok(apps) => apps
                    .into_iter()
                    .map(|(id, app)| (id, Arc::new(app)))
                    .collect(),
                Err(error) => match error {
                    TraceError::PathNotFound(_) => {
                        debug!("Applications file not found, using empty list");
                        HashMap::new()
                    }
                    _ => {
                        error!("Failed to load applications due to {error:?}");
                        return Err(error);
                    }
                },
            };
        debug!(
            "Successfully loaded {} applications from disk.",
            applications.values().len()
        );

        // Load all tasks
        let tasks: HashMap<String, Arc<Task>> = match Task::load_all(storage_folder.clone()).await {
            Ok(tasks) => tasks
                .into_iter()
                .map(|(id, task)| (id, Arc::new(task)))
                .collect(),
            Err(error) => match error {
                TraceError::PathNotFound(_) => {
                    debug!("Tasks file not found, using empty list");
                    HashMap::new()
                }
                _ => {
                    error!("Failed to load applications due to {error:?}");
                    return Err(error);
                }
            },
        };
        debug!(
            "Successfully loaded {} tasks from disk.",
            tasks.values().len()
        );

        Ok(Self {
            storage_folder,
            applications: RwLock::new(applications),
            tasks: RwLock::new(tasks),
        })
    }
}

#[async_trait]
impl Storage for Database {
    async fn applications_read(&self) -> HashMap<Uuid, Arc<Application>> {
        self.applications.read().await.clone()
    }

    async fn applications_write(
        &self,
    ) -> WriteableDataBaseGuard<'_, HashMap<Uuid, Arc<Application>>> {
        let elements = self.applications.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "applications",
            elements,
        }
    }

    async fn tasks_read(&self) -> HashMap<String, Arc<Task>> {
        self.tasks.read().await.clone()
    }

    async fn tasks_write(&self) -> WriteableDataBaseGuard<'_, HashMap<String, Arc<Task>>> {
        let elements = self.tasks.write().await;

        WriteableDataBaseGuard {
            folder: &self.storage_folder,
            title: "tasks",
            elements,
        }
    }
}
