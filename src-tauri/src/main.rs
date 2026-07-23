// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args_os().any(|argument| argument == "--install-codex-hooks") {
        if codex_beacon_lib::install_codex_hooks_for_current_user().is_err() {
            std::process::exit(1);
        }

        return;
    }

    codex_beacon_lib::run()
}
