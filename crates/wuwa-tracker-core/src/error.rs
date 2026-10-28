use thiserror::Error;

#[derive(Debug, Error)]
/// core와 애플리케이션 계층 사이에서 공유하는 실패 유형입니다.
pub enum AppError {
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("invalid echo set data from encore API")]
    InvalidEchoSetData,
    #[error("echo set data is unavailable")]
    EchoSetsUnavailable,
    #[error("missing player id")]
    MissingPlayerId,
    #[error("empty upload data")]
    EmptyUploadData,
    #[error("scan path not found")]
    ScanPathNotFound,
    #[error("log file not found")]
    LogFileNotFound,
    #[error("url not found")]
    UrlNotFound,
    #[error("invalid gacha url or unsupported domain")]
    InvalidGachaUrl,
    #[error("tracker API rejected request: code={code}, message={message}")]
    TrackerRejected { code: i32, message: String },
    #[error("missing url")]
    MissingUrl,
    #[error("unsupported report format: {0}")]
    UnsupportedReportFormat(String),
    #[error("no valid records found")]
    NoValidRecords,
    #[error("player not found")]
    PlayerNotFound,
    #[error("embedded database error: {0}")]
    Database(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Template(#[from] askama::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
