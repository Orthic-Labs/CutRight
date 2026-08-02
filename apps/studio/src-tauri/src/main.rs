mod artifact_state;
mod commands;
mod decision_contract;
mod decision_ledger;
mod decision_store;
mod project_identity;
mod project_scope;
mod relink_history;
mod settings;
mod source_integrity;

#[cfg(test)]
mod tests;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::pick_project,
            commands::read_snapshot,
            commands::read_transcript,
            commands::append_decision,
            commands::read_decisions,
            source_integrity::verify_sources,
            commands::select_variant,
            commands::read_variant_selection,
            source_integrity::relink_source,
            commands::read_cloud_settings,
            commands::write_cloud_settings,
            commands::delete_cloud_data,
            commands::credential_env_var_present,
            commands::read_engine_status
        ])
        .run(tauri::generate_context!())
        .expect("CutRight Studio failed to start");
}
