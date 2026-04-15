mod commands;
mod config;
mod db;
mod deploy_script;
mod paths;
mod ssh;
mod tray;
mod types;

use commands::check::CheckStreams;
use commands::deploy::DeployStreams;
use commands::logs::LogStreams;
use commands::metrics::MetricsStreams;
use commands::step::StepStreams;
use commands::terminal::TerminalSessions;
use db::Db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".into()))
        .init();

    // Migrate legacy overlord/ directories before any code reads config.
    paths::migrate_legacy_dirs();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            tray::init(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // On close-request, hide the window instead of quitting so the tray
            // stays useful. Quit via tray menu or ⌘Q.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .manage(LogStreams::default())
        .manage(DeployStreams::default())
        .manage(CheckStreams::default())
        .manage(StepStreams::default())
        .manage(TerminalSessions::default())
        .manage(MetricsStreams::default())
        .manage(Db::open().expect("failed to open history DB"))
        .invoke_handler(tauri::generate_handler![
            commands::projects::list_projects,
            commands::projects::add_project,
            commands::projects::remove_project,
            commands::projects::ping_env,
            commands::containers::get_containers,
            commands::logs::start_log_stream,
            commands::logs::stop_log_stream,
            commands::deploy::start_deploy,
            commands::deploy::cancel_deploy,
            commands::deploy::restart_service,
            commands::env_vars::get_env_vars,
            commands::env_vars::set_env_var,
            commands::env_vars::delete_env_var,
            commands::history::list_deploys,
            commands::history::get_deploy_log,
            commands::check::start_health_check,
            commands::check::cancel_health_check,
            commands::terminal::start_terminal,
            commands::terminal::term_write,
            commands::terminal::term_resize,
            commands::terminal::term_close,
            commands::metrics::start_metrics,
            commands::metrics::stop_metrics,
            commands::onepassword::op_status,
            commands::onepassword::op_read,
            commands::init_project::init_project,
            commands::init_project::update_remote,
            commands::step::start_cli_step,
            commands::step::cancel_cli_step,
            commands::hosts::list_saved_hosts,
            commands::hosts::add_saved_host,
            commands::hosts::update_saved_host,
            commands::hosts::remove_saved_host,
            commands::hosts::list_ssh_config_hosts,
            commands::scrollback::read_scrollback,
            commands::scrollback::write_scrollback,
            commands::scrollback::clear_scrollback,
            commands::scaffold::has_compose_file,
            commands::scaffold::read_compose_file,
            commands::scaffold::write_compose_file,
            commands::scaffold::has_dockerfile,
            commands::scaffold::read_dockerfile,
            commands::scaffold::write_dockerfile,
            commands::scaffold::has_shared_files,
            commands::scaffold::shared_file_status,
            commands::scaffold::read_shared_compose,
            commands::scaffold::write_shared_compose,
            commands::scaffold::read_shared_traefik,
            commands::scaffold::write_shared_traefik,
            commands::scaffold::read_shared_init_db,
            commands::scaffold::write_shared_init_db,
            commands::scaffold::has_dockerignore,
            commands::scaffold::read_dockerignore,
            commands::scaffold::write_dockerignore,
            tray::refresh_tray,
            tray::set_tray_tooltip,
            deploy_script::install_deploy_script,
            deploy_script::has_local_deploy_script,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
