use askama::Template;

use crate::{config::Config, error::AppError, translations};
use wuwa_tracker_types::{ReportData, Stats};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// 생성 가능한 리포트 직렬화 형식입니다.
pub enum ReportFormat {
    Html,
    Json,
    Csv,
}

impl ReportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Html => "text/html; charset=utf-8",
            Self::Json => "application/json; charset=utf-8",
            Self::Csv => "text/csv; charset=utf-8",
        }
    }
}

impl std::str::FromStr for ReportFormat {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "html" => Ok(Self::Html),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            other => Err(AppError::UnsupportedReportFormat(other.to_string())),
        }
    }
}

/// 통계 데이터를 요청한 형식의 바이트로 직렬화합니다.
///
/// # Errors
///
/// JSON 직렬화, HTML 템플릿 렌더링 또는 번역 로드에 실패하면 [`AppError`]를 반환합니다.
pub fn export(
    config: &Config,
    data: &ReportData,
    format: ReportFormat,
    lang: &str,
) -> Result<Vec<u8>, AppError> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_vec_pretty(data)?),
        ReportFormat::Csv => export_csv(data),
        ReportFormat::Html => export_html(config, data, lang),
    }
}

fn export_csv(data: &ReportData) -> Result<Vec<u8>, AppError> {
    let mut out = String::from(
        "PlayerID,GachaType,GachaName,CardPoolType,ResourceID,QualityLevel,ResourceType,Name,Count,Time\n",
    );
    for stat in &data.stats {
        for rec in &stat.records {
            out.push_str(&csv_row(&[
                data.player_id.clone(),
                stat.gacha_type.to_string(),
                stat.gacha_name.clone(),
                rec.card_pool_type.clone(),
                rec.resource_id.to_string(),
                rec.quality_level.to_string(),
                rec.resource_type.clone(),
                rec.name.clone(),
                rec.count.to_string(),
                rec.time.clone(),
            ]));
            out.push('\n');
        }
    }
    Ok(out.into_bytes())
}

fn csv_row(values: &[String]) -> String {
    values
        .iter()
        .map(|value| {
            if value.contains([',', '"', '\n']) {
                format!("\"{}\"", value.replace('"', "\"\""))
            } else {
                value.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn export_html(config: &Config, data: &ReportData, lang: &str) -> Result<Vec<u8>, AppError> {
    let i18n = translations::load(lang)?;
    let report = ReportTemplate {
        lang: &i18n.lang,
        player_id: &data.player_id,
        stats: &data.stats,
        threshold_count: config.luck_score_thresholds.len(),
        luck_score_unknown: i18n
            .translations
            .get("report.luck_score_unknown")
            .map(String::as_str)
            .unwrap_or("N/A"),
        luck_score_label: i18n
            .translations
            .get("report.luck_score")
            .map(String::as_str)
            .unwrap_or("Luck Score"),
    };

    Ok(report.render()?.into_bytes())
}

#[derive(Template)]
#[template(path = "report.html")]
struct ReportTemplate<'a> {
    lang: &'a str,
    player_id: &'a str,
    stats: &'a [Stats],
    threshold_count: usize,
    luck_score_label: &'a str,
    luck_score_unknown: &'a str,
}
