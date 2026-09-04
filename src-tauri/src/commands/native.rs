use tauri::State;

use crate::native::PendingOpen;

/// Pulled once by the frontend on launch to pick up a file passed via
/// "Open with Foldown" or a second-instance relaunch — see `PendingOpen`.
#[tauri::command]
pub fn take_pending_open(state: State<PendingOpen>) -> Option<String> {
    state.0.lock().unwrap().take()
}
