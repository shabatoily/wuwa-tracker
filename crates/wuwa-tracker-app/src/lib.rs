//! CLI, Tauri와 HTTP 진입점을 공통 애플리케이션 서비스에 연결합니다.

#[cfg(feature = "gui")]
pub mod api;
pub mod cli;
pub mod http;
pub mod logging;
pub mod service;
pub mod settings;
pub mod webui_assets;

use anyhow::Result;
use clap::{Parser, Subcommand};
use service::Service;
use std::{env, path::PathBuf};
use wuwa_tracker_core::Config;

const ENV_DB_PATH: &str = "WUWA_TRACKER_DB_PATH";
const ENV_LOG_PATH: &str = "WUWA_TRACKER_LOG_PATH";

#[derive(Debug, Parser)]
#[command(name = "wuwa-tracker")]
#[command(about = "Wuwa Tracker")]
pub struct Cli {
    #[arg(long = "dbpath", global = true, help = "Local embedded database path")]
    db_path: Option<PathBuf>,
    #[arg(long = "logpath", global = true, help = "Application log file path")]
    log_path: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Run the HTTP API server")]
    Serve(http::ServeArgs),
    #[command(about = "Print the application version")]
    Version,
    #[command(about = "Scan a game directory or log file for a tracking URL")]
    Scan(cli::ScanArgs),
    #[command(about = "Generate a report from a URL or saved JSON file")]
    Report(cli::ReportArgs),
    #[command(about = "Scan or fetch gacha records and generate a report")]
    Run(cli::RunArgs),
    #[command(about = "Periodically scan logs and run when the tracking URL changes")]
    Autorun(cli::AutorunArgs),
    #[command(about = "Manage saved CLI defaults")]
    Config(cli::ConfigArgs),
    #[command(about = "Export the local store to a backup JSON file")]
    Backup(cli::BackupArgs),
    #[command(about = "Merge a backup JSON file into the local store")]
    Merge(cli::MergeArgs),
    #[command(about = "Inspect local store data")]
    Db(cli::DbArgs),
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    if matches!(&cli.command, Some(Command::Version)) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let cfg = build_config(cli.db_path.clone(), cli.log_path.clone());
    if let Some(Command::Config(args)) = &cli.command {
        return cli::config(args.clone(), &cfg);
    }

    let console_level = match &cli.command {
        Some(Command::Serve(_)) => Some("info"),
        Some(_) => Some("error"),
        None => None,
    };
    logging::init(&cfg.log_path, console_level)?;
    let service = Service::new(cfg)?;

    match cli.command {
        Some(Command::Serve(args)) => http::serve(args, service).await,
        Some(Command::Version) => unreachable!("version command is handled before service setup"),
        Some(Command::Scan(args)) => cli::scan(args, service),
        Some(Command::Report(args)) => cli::report(args, service).await,
        Some(Command::Run(args)) => cli::run(args, service).await,
        Some(Command::Autorun(args)) => cli::autorun(args, service).await,
        Some(Command::Config(_)) => unreachable!("config command is handled before service setup"),
        Some(Command::Backup(args)) => cli::backup(args, service),
        Some(Command::Merge(args)) => cli::merge(args, service),
        Some(Command::Db(args)) => cli::db(args, service),
        None => {
            <Cli as clap::CommandFactory>::command()
                .name("wuwa-tracker-cli")
                .print_help()?;
            println!();
            Ok(())
        }
    }
}

pub async fn run_gui(gui: fn(Service) -> Result<()>) -> Result<()> {
    let cfg = build_config(None, None);
    logging::init(&cfg.log_path, None)?;
    let service = Service::new(cfg)?;
    service.prepare_echo_sets().await;
    gui(service)
}

/// CLI 설정과 환경변수 override를 반영한 런타임 설정을 생성합니다.
///
/// 함수 인자가 환경변수보다 우선하며 빈 환경변수는 설정되지 않은 값으로 처리합니다.
pub fn build_config(db_path: Option<PathBuf>, log_path: Option<PathBuf>) -> Config {
    let mut config = Config::default();
    if let Some(db_path) = db_path.or_else(|| get_env(ENV_DB_PATH)) {
        config.db_path = db_path;
    }
    if let Some(log_path) = log_path.or_else(|| get_env(ENV_LOG_PATH)) {
        config.log_path = log_path;
    }
    config
}

fn get_env(key: &str) -> Option<PathBuf> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}
