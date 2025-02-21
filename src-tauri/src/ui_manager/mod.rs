// //! Ui manager care doar primeste update-urile si trimite event catre frontend cand sa re-randeze

// use std::os::macos::raw::stat;

// use tauri::AppHandle;

// use crate::state_manager::state::State;

// pub struct UiManager {
//     app_handle: AppHandle,
// }

// impl UiManager {
//     pub fn new(app_handle: AppHandle) -> Self {
//         Self { app_handle }
//     }

//     pub fn render() {}

//     pub fn refresh(state: &State) {
//         // send event
//         app_handle
//             .emit(
//                 "update:tasks",
//                 self.tasks
//                     .read()
//                     .await
//                     .iter()
//                     .map(|(_, value)| value)
//                     .collect::<Vec<&Arc<Task>>>(),
//             )
//             .ok();
//     }
// }
