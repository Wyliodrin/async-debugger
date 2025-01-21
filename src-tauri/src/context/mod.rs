use std::{collections::HashMap, sync::Arc};
use task::Task;
use tokio::sync::{mpsc::Sender, RwLock};
use uuid::Uuid;

pub mod application;
pub mod connection;
pub mod task;

use application::Application;
use connection::Event;

pub struct Context {
    pub applications: RwLock<HashMap<Uuid, Arc<Application>>>,
    pub event_sender: Sender<(Uuid, Event)>,

    pub tasks: RwLock<HashMap<String, Arc<Task>>>,
}

impl Context {
    pub fn new(event_sender: Sender<(Uuid, Event)>) -> Context {
        Context {
            applications: RwLock::new(HashMap::new()),
            event_sender,
            tasks: RwLock::new(HashMap::new()),
        }
    }
}
