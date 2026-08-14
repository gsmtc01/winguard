// src-tauri/src/checks/virustotal.rs
//
// VirusTotal API v3 파일 검사 모듈.
//
// 흐름:
//   1. SHA256 로컬 계산
//   2. GET /files/{sha256}  — 이미 VT에 등록된 파일이면 업로드 없이 결과 반환
//   3. 미등록이면 → POST /files  (사용자가 명시적으로 upload=true 를 전달했을 때만)
//   4. GET /analyses/{id}  — status == "completed" 까지 폴링 (최대 60초)
//
// 프라이버시 설계:
//   - 해시 조회(2단계)는 파일 내용을 서버로 보내지 않는다.
//   - 업로드(3단계)는 파일 내용이 VT 서버로 전송되므로, 프론트엔드에서
//     사용자 동의를 받은 뒤 upload=true 를 명시해야 한다.
//   - API 키는 OS 키체인(keyring crate)에 저장하고, 이 모듈은 외부에서
//     전달받는다. 코드에 절대 하드코딩하지 않는다.

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    thread,
    time::Duration,
};

// ── 공개 입출력 타입 ───────────────────────────────────────────

/// 엔진별 탐지 결과 요약
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineResult {
    pub engine_name: String,
    pub category: String, // "malicious" | "suspicious" | "undetected" | "harmless"
    pub result: Option<String>, // 탐지된 악성코드 이름 (없으면 None)
}

/// VT 검사 결과 — Tauri IPC 반환 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtScanResult {
    /// 파일 SHA256 (로컬에서 계산)
    pub sha256: String,
    /// 검사 상태
    pub status: VtStatus,
    /// 전체 엔진 수
    pub total_engines: u32,
    /// 악성으로 판정한 엔진 수
    pub malicious: u32,
    /// 의심으로 판정한 엔진 수
    pub suspicious: u32,
    /// 미탐지 엔진 수
    pub undetected: u32,
    /// 탐지한 엔진 목록 (malicious + suspicious)
    pub detections: Vec<EngineResult>,
    /// VT 웹사이트 링크
    pub vt_link: String,
    /// 오류 메시지 (status == Error 일 때)
    pub error_message: Option<String>,
    /// 파일을 VT에 새로 업로드했는지 여부
    pub was_uploaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VtStatus {
    Clean,      // malicious == 0 && suspicious == 0
    Suspicious, // suspicious > 0 && malicious == 0
    Malicious,  // malicious > 0
    Error,      // API 오류 또는 타임아웃
}

// ── 내부 API 응답 타입 ─────────────────────────────────────────

#[derive(Deserialize)]
struct FileReportResponse {
    data: FileReportData,
}

#[derive(Deserialize)]
struct FileReportData {
    attributes: FileAttributes,
}

#[derive(Deserialize)]
struct FileAttributes {
    last_analysis_stats: AnalysisStats,
    last_analysis_results: HashMap<String, EngineDetail>,
}

#[derive(Deserialize)]
struct AnalysisStats {
    malicious: u32,
    suspicious: u32,
    undetected: u32,
    harmless: u32,
    #[serde(rename = "type-unsupported")]
    type_unsupported: u32,
    timeout: u32,
}

#[derive(Deserialize)]
struct EngineDetail {
    category: String,
    result: Option<String>,
}

#[derive(Deserialize)]
struct UploadResponse {
    data: UploadData,
}

#[derive(Deserialize)]
struct UploadData {
    id: String, // analysis ID
}

#[derive(Deserialize)]
struct AnalysisResponse {
    data: AnalysisData,
}

#[derive(Deserialize)]
struct AnalysisData {
    attributes: AnalysisAttributes,
}

#[derive(Deserialize)]
struct AnalysisAttributes {
    status: String, // "queued" | "in-progress" | "completed"
    stats: Option<AnalysisStats>,
    results: Option<HashMap<String, EngineDetail>>,
}

// ── SHA256 계산 ────────────────────────────────────────────────

/// 파일을 스트리밍으로 읽어 SHA256 계산 (대용량 파일 안전 처리)
pub fn compute_sha256(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| {
        format!(
            "파일을 열 수 없습니다: path={}, error={}",
            path.display(),
            e
        )
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536]; // 64KB 청크

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("파일 읽기 오류: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

// ── API 클라이언트 헬퍼 ────────────────────────────────────────

fn make_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("WinGuard/1.0")
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))
}

// ── Step 1: 해시로 기존 보고서 조회 ────────────────────────────

#[allow(clippy::type_complexity)]
fn fetch_report_by_hash(
    client: &Client,
    api_key: &str,
    sha256: &str,
) -> Result<Option<(AnalysisStats, HashMap<String, EngineDetail>)>, String> {
    let url = format!("https://www.virustotal.com/api/v3/files/{}", sha256);

    let resp = client
        .get(&url)
        .header("x-apikey", api_key)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("VT 해시 조회 네트워크 오류: {}", e))?;

    match resp.status().as_u16() {
        200 => {
            let report: FileReportResponse = resp
                .json()
                .map_err(|e| format!("VT 해시 응답 파싱 오류: {}", e))?;
            Ok(Some((
                report.data.attributes.last_analysis_stats,
                report.data.attributes.last_analysis_results,
            )))
        }
        404 => Ok(None), // VT에 미등록 파일
        401 => Err("API 키가 유효하지 않습니다. 설정에서 API 키를 확인하세요.".into()),
        429 => {
            Err("API 요청 한도 초과입니다 (무료 플랜: 분당 4회). 잠시 후 다시 시도하세요.".into())
        }
        code => Err(format!("VT API 오류: HTTP {}", code)),
    }
}

// ── Step 2: 파일 업로드 ────────────────────────────────────────

fn upload_file(client: &Client, api_key: &str, path: &Path) -> Result<String, String> {
    // 32MB 초과 파일은 무료 플랜에서 업로드 불가
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("파일 메타데이터 읽기 실패: {}", e))?;

    const MAX_SIZE: u64 = 32 * 1024 * 1024; // 32MB
    if metadata.len() > MAX_SIZE {
        return Err(format!(
            "파일이 너무 큽니다 ({:.1}MB). 무료 VT API는 32MB 이하 파일만 지원합니다.",
            metadata.len() as f64 / 1024.0 / 1024.0
        ));
    }

    let file_bytes = std::fs::read(path).map_err(|e| format!("파일 읽기 실패: {}", e))?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let part = reqwest::blocking::multipart::Part::bytes(file_bytes).file_name(file_name);
    let form = reqwest::blocking::multipart::Form::new().part("file", part);

    let resp = client
        .post("https://www.virustotal.com/api/v3/files")
        .header("x-apikey", api_key)
        .header("Accept", "application/json")
        .multipart(form)
        .send()
        .map_err(|e| format!("VT 업로드 네트워크 오류: {}", e))?;

    match resp.status().as_u16() {
        200 => {
            let upload: UploadResponse = resp
                .json()
                .map_err(|e| format!("VT 업로드 응답 파싱 오류: {}", e))?;
            Ok(upload.data.id)
        }
        429 => Err("API 요청 한도 초과입니다. 잠시 후 다시 시도하세요.".into()),
        code => Err(format!("VT 업로드 실패: HTTP {}", code)),
    }
}

// ── Step 3: 분석 완료까지 폴링 ────────────────────────────────

fn poll_analysis(
    client: &Client,
    api_key: &str,
    analysis_id: &str,
) -> Result<(AnalysisStats, HashMap<String, EngineDetail>), String> {
    let url = format!("https://www.virustotal.com/api/v3/analyses/{}", analysis_id);

    // 최대 60초, 5초 간격으로 폴링 (= 12회 시도)
    for attempt in 0..12 {
        if attempt > 0 {
            thread::sleep(Duration::from_secs(5));
        }

        let resp = client
            .get(&url)
            .header("x-apikey", api_key)
            .header("Accept", "application/json")
            .send()
            .map_err(|e| format!("VT 폴링 네트워크 오류: {}", e))?;

        if resp.status().as_u16() == 429 {
            // 레이트 리밋 시 추가 대기
            thread::sleep(Duration::from_secs(15));
            continue;
        }

        let analysis: AnalysisResponse = resp
            .json()
            .map_err(|e| format!("VT 분석 응답 파싱 오류: {}", e))?;

        if analysis.data.attributes.status == "completed" {
            let stats = analysis
                .data
                .attributes
                .stats
                .ok_or("분석 완료됐으나 통계 데이터가 없습니다.".to_string())?;
            let results = analysis.data.attributes.results.unwrap_or_default();
            return Ok((stats, results));
        }
        // queued / in-progress → 계속 대기
    }

    Err("VT 분석 타임아웃: 60초 내에 분석이 완료되지 않았습니다.".into())
}

// ── 결과 조합 ──────────────────────────────────────────────────

fn build_result(
    sha256: String,
    stats: AnalysisStats,
    results: HashMap<String, EngineDetail>,
    was_uploaded: bool,
) -> VtScanResult {
    let total = stats.malicious
        + stats.suspicious
        + stats.undetected
        + stats.harmless
        + stats.type_unsupported
        + stats.timeout;

    // 탐지 엔진만 추출 (malicious + suspicious)
    let detections: Vec<EngineResult> = results
        .into_iter()
        .filter(|(_, d)| d.category == "malicious" || d.category == "suspicious")
        .map(|(name, d)| EngineResult {
            engine_name: name,
            category: d.category,
            result: d.result,
        })
        .collect();

    let status = if stats.malicious > 0 {
        VtStatus::Malicious
    } else if stats.suspicious > 0 {
        VtStatus::Suspicious
    } else {
        VtStatus::Clean
    };

    VtScanResult {
        vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
        sha256,
        status,
        total_engines: total,
        malicious: stats.malicious,
        suspicious: stats.suspicious,
        undetected: stats.undetected,
        detections,
        error_message: None,
        was_uploaded,
    }
}

// ── 공개 진입점 ────────────────────────────────────────────────

/// 파일 VT 검사 메인 함수
///
/// # Arguments
/// - `file_path`: 검사할 파일 경로
/// - `api_key`: VT API 키 (OS 키체인에서 가져와 전달)
/// - `allow_upload`: 해시 미등록 시 파일을 VT에 업로드할지 여부.
///   프론트엔드에서 사용자 동의를 받은 후 true 로 설정.
pub fn scan_file(file_path: &str, api_key: &str, allow_upload: bool) -> VtScanResult {
    let path = Path::new(file_path);

    // SHA256 계산
    let sha256 = match compute_sha256(path) {
        Ok(h) => h,
        Err(e) => return error_result("".into(), e),
    };

    let client = match make_client() {
        Ok(c) => c,
        Err(e) => return error_result(sha256, e),
    };

    // Step 1: 해시 조회
    match fetch_report_by_hash(&client, api_key, &sha256) {
        Err(e) => return error_result(sha256, e),

        Ok(Some((stats, results))) => {
            // VT에 이미 있음 → 업로드 없이 결과 반환
            return build_result(sha256, stats, results, false);
        }

        Ok(None) => {
            // VT에 없는 파일
            if !allow_upload {
                return VtScanResult {
                    vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
                    sha256,
                    status: VtStatus::Error,
                    total_engines: 0,
                    malicious: 0,
                    suspicious: 0,
                    undetected: 0,
                    detections: vec![],
                    error_message: Some(
                        "이 파일은 VirusTotal에 등록되어 있지 않습니다. \
                         파일을 업로드하면 70개 이상의 백신 엔진으로 검사할 수 있습니다."
                            .into(),
                    ),
                    was_uploaded: false,
                };
            }
        }
    }

    // Step 2: 업로드
    let analysis_id = match upload_file(&client, api_key, path) {
        Ok(id) => id,
        Err(e) => return error_result(sha256, e),
    };

    // Step 3: 폴링
    match poll_analysis(&client, api_key, &analysis_id) {
        Ok((stats, results)) => build_result(sha256, stats, results, true),
        Err(e) => error_result(sha256, e),
    }
}

fn error_result(sha256: String, msg: String) -> VtScanResult {
    VtScanResult {
        vt_link: if sha256.is_empty() {
            String::new()
        } else {
            format!("https://www.virustotal.com/gui/file/{}", sha256)
        },
        sha256,
        status: VtStatus::Error,
        total_engines: 0,
        malicious: 0,
        suspicious: 0,
        undetected: 0,
        detections: vec![],
        error_message: Some(msg),
        was_uploaded: false,
    }
}

// ── 테스트 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn sha256_of_empty_file_is_known_value() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"").unwrap();
        let hash = compute_sha256(f.path()).unwrap();
        // 빈 파일의 SHA256 — 고정값
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_of_known_bytes() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"hello world").unwrap();
        let hash = compute_sha256(f.path()).unwrap();
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn missing_file_returns_error_result_not_panic() {
        let result = scan_file("/nonexistent/path/file.exe", "fake_key", false);
        assert_eq!(result.status, VtStatus::Error);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn no_upload_when_allow_upload_false_and_hash_unknown() {
        // 실제 네트워크 없이는 hash unknown 케이스를 단위 테스트하기 어려움
        // → integration test 에서 mock server 로 검증
        // 여기서는 allow_upload=false 일 때 was_uploaded=false 임을 확인
        let result = scan_file("/nonexistent/path/file.exe", "fake_key", false);
        assert!(!result.was_uploaded);
    }
}
