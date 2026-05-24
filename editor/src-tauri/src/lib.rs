mod ai;
mod commands;
mod engine_client;
mod engine_process;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(engine_process::EngineProcess(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            // Engine lifecycle
            engine_process::start_engine,
            engine_process::stop_engine,
            engine_process::get_engine_binary_path,
            // Scene + engine HTTP proxy
            engine_client::get_scene,
            engine_client::get_engine_status,
            engine_client::put_scene,
            engine_client::save_scene,
            engine_client::open_scene_file,
            engine_client::patch_transform,
            engine_client::get_screenshot,
            engine_client::get_runtime_errors,
            engine_client::set_engine_paused,
            engine_client::set_engine_playback,
            engine_client::set_gizmos,
            engine_client::get_script,
            engine_client::write_script,
            engine_client::list_scripts,
            engine_client::create_entity,
            engine_client::rename_entity,
            engine_client::add_component,
            engine_client::remove_component,
            engine_client::patch_component,
            // AI — chat
            ai::chat::send_ai_message,
            ai::chat::send_ai_message_stream,
            // AI — provider management
            ai::client::get_ai_provider_status,
            ai::client::save_ai_api_key,
            ai::client::clear_ai_api_key,
            ai::client::test_ai_provider,
            // AI — proposals + actions
            ai::proposal::generate_proposal,
            ai::proposal::apply_action,
            ai::proposal::commit_staged_change,
            ai::proposal::revert_staged_change,
            ai::proposal::clear_staged_proposal,
            ai::proposal::get_pending_proposal,
            // Project management
            commands::create_project,
            commands::list_project_files,
            commands::list_project_tree,
            commands::new_script,
            commands::new_scene_file,
            commands::create_folder,
            commands::move_project_entry,
            commands::delete_project_file,
            commands::rename_project_file,
            commands::write_anim_file,
            commands::create_tile_palette,
            commands::read_tile_palette,
            commands::read_text_file,
            commands::read_project_file,
            commands::get_project_settings,
            commands::save_project_settings,
            commands::get_editor_prefs,
            commands::save_editor_prefs,
            // Prefabs
            commands::list_prefabs,
            commands::save_as_prefab,
            commands::instantiate_prefab,
            commands::update_prefab,
            commands::sync_from_prefab,
            commands::unlink_from_prefab,
            // Ollama suggestions
            commands::list_ollama_models,
            commands::ensure_suggestion_model,
            commands::generate_entity_suggestions,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
