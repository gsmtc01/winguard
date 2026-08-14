use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Ok,
    Info,
    Warning,
    Danger,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub message: String,
    pub action_uri: Option<String>,
    pub evidence: HashMap<String, String>,
    pub checked_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub score: u8,
    pub checks: Vec<CheckResult>,
    pub scanned_at: i64,
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}
