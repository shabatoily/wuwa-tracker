use crate::{service::Service, settings};
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use unicode_width::UnicodeWidthStr;
use wuwa_tracker_core::reporter::ReportFormat;
use wuwa_tracker_types::{FetchResult, StatsResponse};

const DB_BANNERS_ID_WIDTH: usize = 4;
const DB_BANNERS_KEY_WIDTH: usize = 26;
const DB_BANNERS_NAME_WIDTH: usize = 28;
const DB_CHARACTERS_ID_WIDTH: usize = 6;
const DB_CHARACTERS_NAME_WIDTH: usize = 24;

#[derive(Debug, Clone, Args)]
pub struct ScanArgs {
    #[arg(short, long, help = "Game root directory or log file path to scan")]
    pub path: Option<PathBuf>,
    #[arg(long, help = "Copy the scanned URL to the system clipboard")]
    pub clipboard: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ReportArgs {
    #[arg(long, help = "Tracking URL to fetch gacha records from")]
    pub url: Option<String>,
    #[arg(
        short = 'f',
        long = "file",
        help = "FetchResult or legacy JSON file to import before reporting"
    )]
    pub file: Option<PathBuf>,
    #[arg(long, help = "Report format: html, json, or csv")]
    pub format: Option<String>,
    #[arg(short = 'o', long = "output", help = "Output file path or basename")]
    pub output: Option<PathBuf>,
    #[arg(long, help = "Report language code")]
    pub lang: Option<String>,
    #[arg(short = 'v', long, help = "Print progress messages")]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct RunArgs {
    #[arg(long, help = "Tracking URL to fetch gacha records from")]
    pub url: Option<String>,
    #[arg(short, long, help = "Game root directory or log file path to scan")]
    pub path: Option<PathBuf>,
    #[arg(long, help = "Report format: html, json, or csv")]
    pub format: Option<String>,
    #[arg(short = 'o', long = "output", help = "Output file path or basename")]
    pub output: Option<PathBuf>,
    #[arg(long, help = "Report language code")]
    pub lang: Option<String>,
    #[arg(short = 'v', long, help = "Print progress messages")]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct AutorunArgs {
    #[arg(short, long, help = "Game root directory or log file path to scan")]
    pub path: Option<PathBuf>,
    #[arg(
        long,
        visible_alias = "interval",
        help = "Polling interval in seconds; defaults to config"
    )]
    pub interval_secs: Option<u64>,
    #[arg(long, help = "Report format: html, json, or csv")]
    pub format: Option<String>,
    #[arg(short = 'o', long = "output", help = "Output file path or basename")]
    pub output: Option<PathBuf>,
    #[arg(long, help = "Report language code")]
    pub lang: Option<String>,
    #[arg(short = 'v', long, help = "Print progress messages")]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct BackupArgs {
    #[arg(
        short = 'o',
        long = "output",
        default_value = "wuwa-tracker.backup.json",
        help = "Backup JSON output path"
    )]
    pub output: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct MergeArgs {
    #[arg(short = 'f', long = "file", help = "Backup JSON file to merge")]
    pub file: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: Option<DbCommand>,
}

#[derive(Debug, Clone, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Show resolved CLI defaults")]
    Show,
    #[command(about = "Set a CLI default")]
    Set {
        #[arg(help = "Configuration key")]
        key: ConfigKey,
        #[arg(help = "Value to save")]
        value: String,
    },
    #[command(about = "Clear saved CLI defaults")]
    Clear,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConfigKey {
    #[value(name = "scan.path")]
    ScanPath,
    #[value(name = "report.format")]
    ReportFormat,
    #[value(name = "report.output")]
    ReportOutput,
    #[value(name = "report.language")]
    ReportLanguage,
    #[value(name = "autorun.interval")]
    AutorunInterval,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DbCommand {
    #[command(about = "List player IDs stored locally")]
    Players,
    #[command(about = "Show store status or summarized statistics for a player")]
    Stats {
        #[arg(help = "Player ID to inspect; omit to show store status")]
        player_id: Option<String>,
    },
    #[command(about = "Show per-banner record counts for a player")]
    Banners {
        #[arg(help = "Player ID to inspect")]
        player_id: String,
    },
    #[command(about = "Show 5-star character summaries for a player")]
    Characters {
        #[arg(help = "Player ID to inspect")]
        player_id: String,
    },
}

pub fn scan(args: ScanArgs, service: Service) -> Result<()> {
    let user_settings = settings::load(&service.config().settings_path)?;
    let path = resolve_path(args.path, &user_settings)?;
    let response = service.scan(&path)?;
    println!("{}", response.url);
    if args.clipboard {
        copy_to_clipboard(&response.url)?;
        println!("URL copied to clipboard.");
    }
    Ok(())
}

pub async fn report(args: ReportArgs, service: Service) -> Result<()> {
    if args.url.is_some() == args.file.is_some() {
        anyhow::bail!("provide exactly one of --url or --file");
    }
    let user_settings = settings::load(&service.config().settings_path)?;
    let format = resolve_format(args.format, &user_settings);
    let output = resolve_output(args.output, &user_settings);
    let lang = resolve_lang(args.lang, &user_settings);

    let stats = if let Some(file) = args.file {
        let fetch_result = service.load_fetch_result_file(&file)?;
        let player_id = fetch_result.payload.player_id.clone();
        service.upload(fetch_result)?;
        service.get_stats(player_id)?
    } else {
        let url = args.url.expect("checked above");
        if args.verbose {
            println!("Fetching gacha data. Please wait...");
        }
        service.prepare_locale(&lang).await;
        let fetch_result = service.fetch_and_save(&url).await?;
        if args.verbose {
            save_fetch_result_log(&fetch_result)?;
        }
        service.get_stats(fetch_result.payload.player_id)?
    };

    write_report(&service, &stats, &format, &output, &lang)?;
    Ok(())
}

pub async fn run(args: RunArgs, service: Service) -> Result<()> {
    let user_settings = settings::load(&service.config().settings_path)?;
    let url = match args.url {
        Some(url) => url,
        None => {
            let path = resolve_path(args.path, &user_settings)?;
            service.scan(&path)?.url
        }
    };
    fetch_and_write_report(
        &service,
        &url,
        &resolve_format(args.format, &user_settings),
        &resolve_output(args.output, &user_settings),
        &resolve_lang(args.lang, &user_settings),
        args.verbose,
    )
    .await?;
    Ok(())
}

pub async fn autorun(args: AutorunArgs, service: Service) -> Result<()> {
    let user_settings = settings::load(&service.config().settings_path)?;
    let interval = Duration::from_secs(
        args.interval_secs
            .or(user_settings.autorun_interval_secs)
            .unwrap_or(service.config().autorun_interval_secs)
            .max(1),
    );
    let mut last_url = String::new();
    let path = resolve_path(args.path, &user_settings)?;
    let format = resolve_format(args.format, &user_settings);
    let output = resolve_output(args.output, &user_settings);
    let lang = resolve_lang(args.lang, &user_settings);

    loop {
        match service.scan(&path) {
            Ok(response) if response.url != last_url => {
                if args.verbose {
                    println!("New URL detected. Running report.");
                }
                match fetch_and_write_report(
                    &service,
                    &response.url,
                    &format,
                    &output,
                    &lang,
                    args.verbose,
                )
                .await
                {
                    Ok(()) => last_url = response.url,
                    Err(error) => eprintln!("Autorun failed: {error}"),
                }
            }
            Ok(_) => {
                if args.verbose {
                    println!("URL unchanged. Waiting.");
                }
            }
            Err(error) => {
                if args.verbose {
                    eprintln!("Scan failed: {error}");
                }
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = tokio::signal::ctrl_c() => return Ok(()),
        }
    }
}

pub fn config(args: ConfigArgs, config: &wuwa_tracker_core::Config) -> Result<()> {
    match args.command.unwrap_or(ConfigCommand::Show) {
        ConfigCommand::Show => show_config(config),
        ConfigCommand::Clear => {
            settings::clear(&config.settings_path)?;
            println!("Settings cleared: {}", config.settings_path.display());
            Ok(())
        }
        ConfigCommand::Set { key, value } => {
            let mut user_settings = settings::load(&config.settings_path)?;
            set_config_value(&mut user_settings, key, value)?;
            settings::save(&config.settings_path, &user_settings)?;
            println!("Settings saved: {}", config.settings_path.display());
            Ok(())
        }
    }
}

fn set_config_value(
    settings: &mut settings::Settings,
    key: ConfigKey,
    value: String,
) -> Result<()> {
    match key {
        ConfigKey::ScanPath => settings.scan_path = Some(value.into()),
        ConfigKey::ReportFormat => {
            ReportFormat::from_str(&value)?;
            settings.report_format = Some(value);
        }
        ConfigKey::ReportOutput => settings.report_output = Some(value.into()),
        ConfigKey::ReportLanguage => settings.report_language = Some(value),
        ConfigKey::AutorunInterval => {
            settings.autorun_interval_secs =
                Some(value.parse().context(
                    "autorun.interval must be a non-negative integer number of seconds",
                )?);
        }
    }
    Ok(())
}

fn show_config(config: &wuwa_tracker_core::Config) -> Result<()> {
    let user_settings = settings::load(&config.settings_path)?;
    println!("Settings: {}", config.settings_path.display());
    print_setting(
        "scan.path",
        user_settings
            .scan_path
            .as_ref()
            .map(|path| (path.display().to_string(), "custom"))
            .unwrap_or_else(|| ("(unset)".to_string(), "default")),
    );
    print_setting(
        "report.format",
        user_settings
            .report_format
            .clone()
            .map(|value| (value, "custom"))
            .unwrap_or_else(|| (settings::DEFAULT_FORMAT.to_string(), "default")),
    );
    print_setting(
        "report.output",
        user_settings
            .report_output
            .as_ref()
            .map(|path| (path.display().to_string(), "custom"))
            .unwrap_or_else(|| (settings::DEFAULT_OUTPUT.to_string(), "default")),
    );
    print_setting(
        "report.language",
        user_settings
            .report_language
            .clone()
            .map(|value| (value, "custom"))
            .unwrap_or_else(|| (settings::DEFAULT_LANG.to_string(), "default")),
    );
    print_setting(
        "autorun.interval",
        user_settings
            .autorun_interval_secs
            .map(|value| (value.to_string(), "custom"))
            .unwrap_or_else(|| (config.autorun_interval_secs.to_string(), "default")),
    );
    Ok(())
}

fn print_setting(name: &str, value: (String, &str)) {
    println!("{}: {} ({})", name, value.0, value.1);
}

fn resolve_path(path: Option<PathBuf>, user_settings: &settings::Settings) -> Result<PathBuf> {
    path.or_else(|| user_settings.scan_path.clone())
        .context("provide --path or save one with `wuwa-tracker config set scan.path <PATH>`")
}

fn resolve_format(format: Option<String>, user_settings: &settings::Settings) -> String {
    format
        .or_else(|| user_settings.report_format.clone())
        .unwrap_or_else(|| settings::DEFAULT_FORMAT.to_string())
}

fn resolve_output(output: Option<PathBuf>, user_settings: &settings::Settings) -> PathBuf {
    output
        .or_else(|| user_settings.report_output.clone())
        .unwrap_or_else(|| PathBuf::from(settings::DEFAULT_OUTPUT))
}

fn resolve_lang(lang: Option<String>, user_settings: &settings::Settings) -> String {
    lang.or_else(|| user_settings.report_language.clone())
        .unwrap_or_else(|| settings::DEFAULT_LANG.to_string())
}

pub fn backup(args: BackupArgs, service: Service) -> Result<()> {
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, service.export_backup()?)?;
    println!("Backup created: {}", args.output.display());
    Ok(())
}

pub fn merge(args: MergeArgs, service: Service) -> Result<()> {
    service.merge_backup(&args.file)?;
    println!("Backup merged: {}", args.file.display());
    Ok(())
}

pub fn db(args: DbArgs, service: Service) -> Result<()> {
    match args.command.unwrap_or(DbCommand::Stats { player_id: None }) {
        DbCommand::Stats { player_id } => {
            if let Some(player_id) = player_id {
                let stats = service.get_stats(player_id)?;
                println!("{}", serde_json::to_string_pretty(&stats_summary(&stats)?)?);
            } else {
                let stats = service.store_stats()?;
                println!("DB Stats");
                println!("Path: {}", stats.path.display());
                println!("Exists: {}", stats.exists);
                println!(
                    "Size: {} ({} bytes)",
                    format_bytes(stats.size_bytes),
                    stats.size_bytes
                );
                println!("Players: {}", stats.players);
                println!("Banners: {}", stats.banners);
                println!("Records: {}", stats.records);
            }
        }
        DbCommand::Players => {
            for player in service.list_players() {
                println!("{player}");
            }
        }
        DbCommand::Banners { player_id } => {
            let counts = service.banner_record_counts(player_id)?;
            print_db_banners_row("ID", "Key", "Name", "Count");
            for count in counts {
                print_db_banners_row(
                    &count.id.to_string(),
                    &count.key,
                    &count.name,
                    &count.records.to_string(),
                );
            }
        }
        DbCommand::Characters { player_id } => {
            let summaries = service.character_summaries(player_id)?;
            print_db_characters_row("ID", "Name", "Count", "Astrite", "Last", "Banners");
            for summary in summaries {
                print_db_characters_row(
                    &summary.resource_id.to_string(),
                    &summary.name,
                    &summary.copies.to_string(),
                    &format_number(summary.spent_astrite),
                    &summary.last_time,
                    &summary.banners.join(", "),
                );
            }
        }
    }
    Ok(())
}

async fn fetch_and_write_report(
    service: &Service,
    url: &str,
    format: &str,
    output: &Path,
    lang: &str,
    verbose: bool,
) -> Result<()> {
    service.prepare_locale(lang).await;
    let fetch_result = service.fetch_and_save(url).await?;
    if verbose {
        save_fetch_result_log(&fetch_result)?;
    }
    let stats = service.get_stats(fetch_result.payload.player_id)?;
    write_report(service, &stats, format, output, lang)
}

fn stats_summary(stats: &StatsResponse) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(stats)?;
    if let Some(items) = value
        .get_mut("stats")
        .and_then(serde_json::Value::as_array_mut)
    {
        for item in items {
            if let Some(object) = item.as_object_mut() {
                object.remove("records");
            }
        }
    }
    Ok(value)
}

fn print_db_banners_row(id: &str, key: &str, name: &str, count: &str) {
    println!(
        "{} {} {} {}",
        pad_display(id, DB_BANNERS_ID_WIDTH),
        pad_display(key, DB_BANNERS_KEY_WIDTH),
        pad_display(name, DB_BANNERS_NAME_WIDTH),
        count
    );
}

fn print_db_characters_row(
    id: &str,
    name: &str,
    copies: &str,
    astrite: &str,
    last: &str,
    banners: &str,
) {
    println!(
        "{} {} {} {} {} {}",
        pad_display(id, DB_CHARACTERS_ID_WIDTH),
        pad_display(name, DB_CHARACTERS_NAME_WIDTH),
        pad_display(copies, 6),
        pad_display(astrite, 10),
        pad_display(last, 20),
        banners
    );
}

fn pad_display(value: &str, width: usize) -> String {
    let display_width = UnicodeWidthStr::width(value);
    let padding = width.saturating_sub(display_width);
    format!("{value}{}", " ".repeat(padding))
}

fn write_report(
    service: &Service,
    stats: &StatsResponse,
    format: &str,
    output: &Path,
    lang: &str,
) -> Result<()> {
    let format = ReportFormat::from_str(format)?;
    let mut output = output.to_path_buf();
    if output.extension().and_then(|value| value.to_str()) != Some(format.extension()) {
        output.set_extension(format.extension());
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let content = service.export_report(&stats.player_id, format, lang)?;
    fs::write(&output, content)?;
    println!("Report successfully generated! File: {}", output.display());
    Ok(())
}

fn save_fetch_result_log(fetch_result: &FetchResult) -> Result<()> {
    fs::create_dir_all("logs")?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let path = PathBuf::from("logs").join(format!(
        "{}-{}.json",
        fetch_result.payload.player_id, timestamp
    ));
    fs::write(path, serde_json::to_vec_pretty(fetch_result)?)?;
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const UNIT: f64 = 1024.0;
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    for suffix in ["KB", "MB", "GB", "TB"] {
        value /= UNIT;
        if value < UNIT {
            return trim_float(value, suffix);
        }
    }
    trim_float(value / UNIT, "PB")
}

fn trim_float(value: f64, suffix: &str) -> String {
    let value = format!("{value:.2}");
    format!(
        "{} {suffix}",
        value.trim_end_matches('0').trim_end_matches('.')
    )
}

fn format_number(value: usize) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(character);
    }
    formatted
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("pbcopy");

    #[cfg(target_os = "windows")]
    let mut command = Command::new("clip");

    #[cfg(target_os = "linux")]
    let mut command = {
        if command_exists("xclip") {
            let mut command = Command::new("xclip");
            command.args(["-selection", "clipboard"]);
            command
        } else if command_exists("wl-copy") {
            Command::new("wl-copy")
        } else {
            anyhow::bail!("required utilities (xclip or wl-copy) not found");
        }
    };

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    anyhow::bail!("unsupported clipboard platform");

    let mut child = command.stdin(Stdio::piped()).spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("clipboard command failed");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wuwa_tracker_types::Stats;

    #[test]
    fn config_set_updates_one_named_value() {
        let mut settings = settings::Settings::default();

        set_config_value(&mut settings, ConfigKey::ReportFormat, "json".to_string()).unwrap();

        assert_eq!(settings.report_format.as_deref(), Some("json"));
        assert!(settings.scan_path.is_none());
        assert!(set_config_value(
            &mut settings,
            ConfigKey::AutorunInterval,
            "invalid".to_string()
        )
        .is_err());
    }

    #[test]
    fn stats_summary_omits_raw_records() {
        let response = StatsResponse {
            character_summaries: Vec::new(),
            success: true,
            player_id: "123456789".to_string(),
            error: None,
            error_key: None,
            stats: vec![Stats {
                gacha_type: 1,
                gacha_name: "Featured Resonator Convene".to_string(),
                total_pulls: 10,
                total_astrite: 1_600,
                current_pity5: 10,
                current_pity4: 0,
                base_rate: 0.008,
                expected_pulls: 80,
                five_stars: Vec::new(),
                records: Vec::new(),
                avg_pulls: 0.0,
                actual_rate: 0.0,
                luck_score: 0.0,
                has_five_star: false,
            }],
        };

        let value = stats_summary(&response).unwrap();

        assert_eq!(value["playerId"], "123456789");
        assert_eq!(value["stats"][0]["totalPulls"], 10);
        assert!(value["stats"][0].get("records").is_none());
    }
}
