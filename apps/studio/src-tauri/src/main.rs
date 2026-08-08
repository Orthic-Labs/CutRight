mod artifact_state;
mod commands;
mod decision_contract;
mod decision_ledger;
mod decision_store;
mod macos_player;
mod native_media;
mod pack_commands;
mod privacy_settings;
mod project_identity;
mod project_index;
mod project_scope;
mod relink_history;
mod security_scoped_bookmarks;
mod settings;
mod source_integrity;

#[cfg(test)]
mod tests;

fn main() {
    tauri::Builder::default()
        .manage(native_media::NativeMediaState::default())
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
            commands::finish_commit_variant,
            commands::finish_read_selection,
            source_integrity::relink_source,
            commands::read_cloud_settings,
            commands::write_cloud_settings,
            commands::delete_cloud_data,
            commands::credential_env_var_present,
            commands::read_engine_status,
            commands::rightkit_app_info,
            commands::rightkit_logs_write,
            commands::rightkit_logs_collect,
            commands::rightkit_logs_clear,
            commands::apply::apply_action_batch,
            macos_player::native_player_create,
            macos_player::native_player_load,
            macos_player::native_player_seek,
            macos_player::native_player_play,
            macos_player::native_player_pause,
            macos_player::native_player_attach,
            macos_player::native_player_resize,
            macos_player::native_player_detach,
            macos_player::native_player_set_rate,
            macos_player::native_player_set_volume,
            macos_player::native_player_current_time,
            macos_player::native_player_duration,
            macos_player::native_player_destroy,
            security_scoped_bookmarks::create_security_scoped_bookmark,
            security_scoped_bookmarks::resolve_security_scoped_bookmark,
            security_scoped_bookmarks::release_security_scoped_bookmark,
            native_media::native_media_capabilities,
            native_media::native_media_inspect_asset,
            native_media::native_media_analyze_frames,
            native_media::native_media_render_caption,
            native_media::native_media_render_preview,
            native_media::native_media_audio_features,
            native_media::native_media_cancel
        ])
        .run(tauri::generate_context!())
        .expect("CutRight Studio failed to start");
}
