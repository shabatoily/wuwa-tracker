use anyhow::Result;
use wuwa_tracker::service::Service;

#[tokio::main]
async fn main() -> Result<()> {
    wuwa_tracker::run_gui(run_gui).await
}

fn run_gui(service: Service) -> Result<()> {
    tauri::Builder::default()
        .manage(service)
        .invoke_handler(tauri::generate_handler![
            wuwa_tracker::api::get_config,
            wuwa_tracker::api::list_players,
            wuwa_tracker::api::list_echo_sets,
            wuwa_tracker::api::get_stats,
            wuwa_tracker::api::scan_url,
            wuwa_tracker::api::track_url,
            wuwa_tracker::api::upload_json,
            wuwa_tracker::api::get_i18n,
            wuwa_tracker::api::export_report,
            wuwa_tracker::api::export_backup,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
