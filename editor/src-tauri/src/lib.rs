mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::EngineProcess(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::get_scene,
            commands::get_engine_status,
            commands::put_scene,
            commands::save_scene,
            commands::open_scene_file,
            commands::patch_transform,
            commands::get_screenshot,
            commands::get_runtime_errors,
            commands::send_ai_message,
            commands::create_entity,
            commands::apply_action,
            commands::get_script,
            commands::write_script,
            commands::list_scripts,
            commands::list_ollama_models,
            commands::generate_proposal,
            commands::ensure_suggestion_model,
            commands::generate_entity_suggestions,
            commands::rename_entity,
            commands::add_component,
            commands::patch_component,
            commands::remove_component,
            commands::start_engine,
            commands::stop_engine,
            commands::get_engine_binary_path,
            commands::create_project,
            commands::set_engine_paused,
            commands::set_engine_playback,
            commands::list_project_files,
            commands::new_script,
            commands::delete_project_file,
            commands::rename_project_file,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
