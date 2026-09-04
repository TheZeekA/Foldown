mod ai;
mod commands;
mod convert;
mod error;
mod fs;
mod history;
mod native;
mod search;
mod settings;
mod workspace_authority;

use tauri::{Emitter, Manager, WindowEvent};

use ai::commands::AiRuntime;
use ai::index::KnowledgeIndex;
use history::HistoryStore;
use fs::watcher::{FileWatcher, WorkspaceWatcher};
use native::PendingOpen;
use search::index::SearchIndex;
use settings::store::{SettingsStore, WindowState};
use workspace_authority::ActiveWorkspace;

pub const OPEN_FILE_REQUEST_EVENT: &str = "open-file-request";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(path) = native::extract_md_arg(&args) {
                let path_str = path.to_string_lossy().into_owned();
                *app.state::<PendingOpen>().0.lock().unwrap() = Some(path_str.clone());
                let _ = app.emit(OPEN_FILE_REQUEST_EVENT, path_str);
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let db_path = app.path().app_data_dir()?.join("foldown.db");
            let store = SettingsStore::open(db_path.clone())?;
            app.manage(store);
            app.manage(KnowledgeIndex::open(db_path.clone())?);
            app.manage(HistoryStore::open(db_path.clone())?);
            app.manage(AiRuntime::default());
            app.manage(FileWatcher::new());
            app.manage(WorkspaceWatcher::new());
            app.manage(SearchIndex::new());
            app.manage(ActiveWorkspace::default());

            let pending_open = native::extract_md_arg(&std::env::args().collect::<Vec<_>>())
                .map(|p| p.to_string_lossy().into_owned());
            app.manage(PendingOpen(std::sync::Mutex::new(pending_open)));

            if let Some(window) = app.get_webview_window("main") {
                let store = app.state::<SettingsStore>();
                if let Ok(Some(state)) = store.get_window_state() {
                    let _ =
                        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                            x: state.x,
                            y: state.y,
                        }));
                    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                        width: state.width,
                        height: state.height,
                    }));
                    if state.maximized {
                        let _ = window.maximize();
                    }
                }

                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { .. } = event {
                        if let Some(win) = handle.get_webview_window("main") {
                            let maximized = win.is_maximized().unwrap_or(false);
                            let position = win.outer_position().unwrap_or_default();
                            let size = win.outer_size().unwrap_or_default();
                            let store = handle.state::<SettingsStore>();
                            let _ = store.set_window_state(&WindowState {
                                x: position.x,
                                y: position.y,
                                width: size.width,
                                height: size.height,
                                maximized,
                            });
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_recent_workspaces,
            commands::settings::open_workspace,
            commands::settings::remove_recent_workspace,
            commands::settings::get_theme,
            commands::settings::set_theme,
            commands::settings::get_editor_font,
            commands::settings::set_editor_font,
            commands::settings::get_ai_settings,
            commands::settings::set_ai_settings,
            commands::workspace::get_tree,
            commands::workspace::create_workspace,
            commands::files::read_file,
            commands::files::save_file,
            commands::files::watch_file,
            commands::files::unwatch_file,
            commands::files::watch_workspace,
            commands::files::create_file,
            commands::files::create_folder,
            commands::files::move_path,
            commands::files::delete_path,
            commands::files::duplicate_path,
            commands::files::import_file,
            commands::history::record_history_snapshot,
            commands::history::list_history,
            commands::history::get_history_content,
            commands::history::delete_history_snapshot,
            commands::history::clear_history,
            commands::history::restore_history_snapshot,
            commands::search::index_workspace,
            commands::search::search_workspace,
            commands::convert::convert_document,
            commands::convert::bulk_convert_documents,
            commands::native::take_pending_open,
            ai::commands::send_ai_message,
            ai::commands::run_selection_ai,
            ai::commands::cancel_ai_request,
            ai::commands::rebuild_ai_index,
            ai::commands::preview_ai_retrieval,
            ai::commands::apply_ai_proposal,
            ai::commands::reject_ai_proposal,
            ai::commands::list_ai_models,
            ai::commands::probe_ai_endpoints,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
