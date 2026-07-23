mod codex;
mod desktop;

pub use codex::install_codex_hooks_for_current_user;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = desktop::show_island(app);
        }))
        .setup(desktop::setup)
        .invoke_handler(tauri::generate_handler![
            desktop::set_island_interaction,
            desktop::show_ready_island,
            desktop::minimize_island,
            desktop::get_launch_at_startup,
            desktop::set_launch_at_startup,
            codex::status::get_codex_status,
            codex::status::open_codex_thread,
            codex::status::clear_codex_status,
            codex::integration::get_codex_integration_status,
            codex::integration::install_codex_hooks
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Beacon");
}
