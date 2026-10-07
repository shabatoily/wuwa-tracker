//! 뽑기 기록 수집, 병합, 통계 계산과 로컬 저장을 제공하는 도메인 계층입니다.

pub mod characters;
pub mod config;
pub mod error;
pub mod logger;
pub mod merge;
pub mod reporter;
pub mod scanner;
pub mod stats;
pub mod store;
pub mod tracker;
pub mod translations;

pub use config::Config;
pub use error::AppError;
