#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use broadcast_daw_studio_lib::commands::{self, AppState};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_audio_devices,
            commands::start_audio_engine,
            commands::stop_audio_engine,
            commands::get_audio_stats,
            commands::get_patchbay,
            commands::add_node,
            commands::remove_node,
            commands::connect_nodes,
            commands::disconnect_nodes,
            commands::set_node_bypass,
            commands::scan_vst3_plugins,
            commands::load_vst3_plugin,
            commands::set_vst3_parameter,
            commands::update_eq_bands,
            commands::update_compressor_params,
            commands::set_master_volume,
            commands::scan_midi_devices,
            commands::connect_midi_input,
            commands::disconnect_midi,
            commands::active_midi_device,
            commands::add_midi_mapping,
            commands::get_midi_mappings,
            commands::enable_ltc_decoding,
            commands::disable_ltc_decoding,
            commands::start_playback,
            commands::stop_playback,
            commands::get_current_timecode,
            commands::add_timecode_trigger,
            commands::remove_timecode_trigger,
            commands::get_timecode_triggers,
            commands::set_ducking_params,
            commands::start_multicast_broadcast,
            commands::stop_multicast_broadcast,
            commands::get_sync_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
