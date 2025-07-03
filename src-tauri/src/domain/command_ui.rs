use uuid::Uuid;

pub enum CommandUI {
    UpdateApplications,
    UpdateTasks,
    TryingToConnect { app_id: Uuid },
    Connected { app_id: Uuid },
    FailedConnection { app_id: Uuid },
}
