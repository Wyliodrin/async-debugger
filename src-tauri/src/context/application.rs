use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};
use tauri::Url;
use uuid::Uuid;

use super::{
    connection::{self, Command, Connection, Event},
    Context,
};

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub(crate) enum State {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Application {
    id: Uuid,
    title: String,
    url: Url,
    state: State,

    #[serde(skip)]
    connection: Option<Connection>,
}

impl Application {
    pub fn new(title: String, url: Url) -> Application {
        Application {
            id: Uuid::new_v4(),
            title,
            url,
            state: State::Disabled,

            connection: None,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub async fn enable(&mut self, event_sender: Sender<(Uuid, Event)>) {
        if self.state == State::Disabled {
            self.state = State::Enabled;
            self.connection = Some(connection::start(self.id, self.url.clone(), event_sender));
        }
    }

    pub async fn disable(&mut self) {
        if let Some(connection) = self.connection.take() {
            connection.commands.send(Command::Disconnect).await;
            self.state = State::Disabled;
        }
    }
}

enum ApplicationStatus {
    Disabled,
    Enabled,
}

impl Context {
    pub async fn add_application(&self, title: String, url: Url) -> Uuid {
        let mut applications = self.applications.write().await;
        let application = Arc::new(Application::new(title, url));
        applications.insert(application.id().to_owned(), application.clone());
        application.id
    }

    pub async fn applications(&self) -> Vec<Arc<Application>> {
        (*self.applications.read().await)
            .values()
            .cloned()
            .collect()
    }

    pub async fn delete_connection(&self) -> Result<Application> {
        todo!()
    }
}
