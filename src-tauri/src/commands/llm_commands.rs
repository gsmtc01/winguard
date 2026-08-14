use crate::commands::browser_commands::BrowserExtension;
use crate::commands::device_commands::{AppAccess, DeviceStatus};
use crate::commands::eventlog_commands::EventLogEntry;
use crate::commands::network_commands::NetworkConn;
use crate::commands::process_commands::ProcessInfo;
use crate::engine::types::{CheckResult, ScanReport, Severity};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::pin::pin;
use tauri::Emitter;

const MODEL_URL: &str =
    "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf";

/// 디스크에 저장할 파일명.
///
/// 업스트림(HuggingFace) 파일명을 그대로 쓰면 배포처가 파일명을 바꾸거나
/// 다른 양자화본으로 갈아탈 때마다 로컬 경로가 흔들린다. WinGuard 가 관리하는
/// 파일이라는 게 드러나도록 제품명을 앞에 두고, 어떤 모델·양자화인지 알아볼 수
/// 있게 `<제품>-ai-<모델>-<양자화>.gguf` 규격으로 고정한다.
/// 사용자가 직접 이름을 정하지 않으며, 저장 "폴더"만 고를 수 있다.
const MODEL_FILENAME: &str = "winguard-ai-gemma4-e2b-q4_k_m.gguf";

/// 예전 버전이 업스트림 파일명 그대로 저장해 둔 파일. 이미 받아둔 모델을
/// 다시 내려받게 하지 않으려고 인식만 한다(새로 저장할 때는 쓰지 않는다).
const LEGACY_MODEL_FILENAMES: &[&str] = &["gemma-4-E2B-it-Q4_K_M.gguf"];

/// UI 표기용 모델 이름과 실제 다운로드 크기(HTTP Content-Length 기준).
const MODEL_DISPLAY_NAME: &str = "Gemma 4 E2B Instruct · Q4_K_M";
const MODEL_SIZE_BYTES: u64 = 3_106_738_272;

const SETTINGS_FILENAME: &str = "llm_settings.json";

const N_CTX: u32 = 8192;
const N_GENERATE: i32 = 1024;
const N_GENERATE_REPORT: i32 = 2048;

// ── 전역 모델 상태 ─────────────────────────────────────────────

struct ModelHolder {
    backend: LlamaBackend,
    model: LlamaModel,
}

static MODEL: std::sync::Mutex<Option<ModelHolder>> = std::sync::Mutex::new(None);

// ── 경로 / 설정 ────────────────────────────────────────────────

fn install_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("C:\\Program Files\\WinGuard"))
}

fn settings_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("WinGuard")
        .join(SETTINGS_FILENAME)
}

fn load_model_path_from_settings() -> Option<PathBuf> {
    let content = std::fs::read_to_string(settings_path()).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v["model_path"].as_str().map(PathBuf::from)
}

fn save_model_path_to_settings(path: &str) -> Result<(), String> {
    let sp = settings_path();
    if let Some(parent) = sp.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::json!({ "model_path": path });
    std::fs::write(&sp, json.to_string()).map_err(|e| e.to_string())
}

/// 실제로 사용할 모델 파일 경로.
///
/// ① 설정에 저장된 경로에 파일이 있으면 그것
/// ② 없으면 기본 폴더의 규격 파일명
/// ③ 그것도 없으면 기본 폴더의 옛 파일명(구버전 설치본 재사용)
/// ④ 전부 없으면 "있어야 할 위치"를 돌려준다(오류 메시지에 경로가 찍히도록)
fn active_model_path() -> PathBuf {
    let from_settings = load_model_path_from_settings();
    if let Some(p) = &from_settings {
        if p.exists() {
            return p.clone();
        }
    }

    let dir = install_dir();
    let canonical = dir.join(MODEL_FILENAME);
    if canonical.exists() {
        return canonical;
    }
    for legacy in LEGACY_MODEL_FILENAMES {
        let p = dir.join(legacy);
        if p.exists() {
            return p;
        }
    }

    from_settings.unwrap_or(canonical)
}

// ── 응답 타입 ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct LlmAnalysis {
    pub summary: String,
}

#[derive(Serialize, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
    percent: u8,
}

/// 프런트엔드가 파일명·용량을 하드코딩하지 않도록 한곳에서 내려준다.
#[derive(Serialize, Clone)]
pub struct ModelInfo {
    /// 디스크에 저장되는 파일명 (사용자가 바꾸지 않는다)
    pub file_name: String,
    /// UI 표기용 모델 이름
    pub display_name: String,
    /// 예상 다운로드 크기 (bytes)
    pub size_bytes: u64,
    /// 기본 저장 폴더
    pub default_dir: String,
    pub url: String,
}

// ── 커맨드: 경로 / 상태 조회 ───────────────────────────────────

#[tauri::command]
pub fn llm_model_exists() -> bool {
    active_model_path().exists()
}

#[tauri::command]
pub fn llm_model_path() -> String {
    active_model_path().to_string_lossy().to_string()
}

#[tauri::command]
pub fn llm_default_model_path() -> String {
    install_dir()
        .join(MODEL_FILENAME)
        .to_string_lossy()
        .to_string()
}

/// 기본 저장 "폴더". 파일명은 WinGuard 가 정하므로 폴더만 고르게 한다.
#[tauri::command]
pub fn llm_default_model_dir() -> String {
    install_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn llm_model_info() -> ModelInfo {
    ModelInfo {
        file_name: MODEL_FILENAME.to_string(),
        display_name: MODEL_DISPLAY_NAME.to_string(),
        size_bytes: MODEL_SIZE_BYTES,
        default_dir: install_dir().to_string_lossy().to_string(),
        url: MODEL_URL.to_string(),
    }
}

#[tauri::command]
pub fn llm_model_url() -> String {
    MODEL_URL.to_string()
}

#[tauri::command]
pub fn llm_set_model_path(path: String) -> Result<(), String> {
    save_model_path_to_settings(&path)
}

// ── 커맨드: 다운로드 ───────────────────────────────────────────

/// 모델을 내려받는다.
///
/// 사용자는 저장할 **폴더**만 고르고, 파일명은 `MODEL_FILENAME` 으로 고정된다.
/// 받는 동안에는 `.part` 확장자로 쓰다가 정상 종료 시에만 최종 이름으로 바꾼다
/// (중간에 끊긴 파일을 "설치됨"으로 오인하지 않도록).
#[tauri::command]
pub async fn llm_download_model(app: tauri::AppHandle, dest_dir: String) -> Result<(), String> {
    let dir = if dest_dir.trim().is_empty() {
        install_dir()
    } else {
        PathBuf::from(&dest_dir)
    };

    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "저장 폴더를 만들 수 없습니다: {} ({e})",
            dir.to_string_lossy()
        )
    })?;

    let path = dir.join(MODEL_FILENAME);
    let tmp_path = dir.join(format!("{MODEL_FILENAME}.part"));

    let app_c = app.clone();
    let path_c = path.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(7200))
            .build()
            .map_err(|e| e.to_string())?;

        let mut response = client
            .get(MODEL_URL)
            .send()
            .map_err(|e| format!("다운로드 요청 실패: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("서버 응답 오류: HTTP {}", response.status()));
        }

        let total = response.content_length().unwrap_or(0);
        let mut file =
            std::fs::File::create(&tmp_path).map_err(|e| format!("파일 생성 실패: {e}"))?;

        let mut downloaded: u64 = 0;
        let mut buf = vec![0u8; 64 * 1024];

        let result = (|| -> Result<(), String> {
            loop {
                let n = response.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                downloaded += n as u64;
                let percent = if total > 0 {
                    ((downloaded as f64 / total as f64) * 100.0) as u8
                } else {
                    0
                };
                let _ = app_c.emit(
                    "llm:download-progress",
                    DownloadProgress {
                        downloaded,
                        total,
                        percent,
                    },
                );
            }

            if total > 0 && downloaded != total {
                return Err(format!(
                    "다운로드가 중간에 끊겼습니다: {downloaded}/{total} bytes. 네트워크를 확인하고 다시 시도하세요"
                ));
            }
            Ok(())
        })();

        drop(file);

        if let Err(e) = result {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }

        // 같은 폴더 안 rename 이라 원자적이다. 이전 파일이 있으면 덮어쓴다.
        std::fs::rename(&tmp_path, &path_c).map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            format!("파일 저장 실패: {e}")
        })?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    // 실제로 저장된 경로만 설정에 기록한다(실패 시 잘못된 경로가 남지 않도록).
    save_model_path_to_settings(&path.to_string_lossy())
}

// ── 공통 추론 헬퍼 ─────────────────────────────────────────────
//
// 모든 LLM 커맨드가 공유하는 추론 로직.
// 모델 로드 → llm:phase 이벤트 → 토큰 루프 → 출력 반환.

/// temperature = 0.0 → greedy (결정적, 분석/로드맵에 적합)
/// temperature > 0.0 → 확률 샘플링 (Q&A처럼 질문마다 다른 답변이 필요할 때)
/// n_gen: 최대 생성 토큰 수 (기본 N_GENERATE, 보고서는 N_GENERATE_REPORT 사용)
async fn run_inference(
    app: tauri::AppHandle,
    prompt: String,
    temperature: f32,
) -> Result<String, String> {
    run_inference_n(app, prompt, temperature, N_GENERATE).await
}

async fn run_inference_n(
    app: tauri::AppHandle,
    prompt: String,
    temperature: f32,
    n_gen: i32,
) -> Result<String, String> {
    let path = active_model_path();
    if !path.exists() {
        return Err(format!("모델 파일이 없습니다: {}", path.to_string_lossy()));
    }

    let app_c = app.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let mut guard = MODEL.lock().map_err(|e| e.to_string())?;

        // ── 모델 로드 (필요 시) ──────────────────────────────────
        if guard.is_none() {
            let _ = app_c.emit("llm:phase", "loading");

            let backend = LlamaBackend::init().map_err(|e| format!("백엔드 초기화 실패: {e}"))?;

            let model_params = LlamaModelParams::default();
            let model_params = pin!(model_params);

            let model = LlamaModel::load_from_file(&backend, &path, &model_params)
                .map_err(|e| format!("모델 로드 실패: {e}"))?;

            *guard = Some(ModelHolder { backend, model });
        }

        let _ = app_c.emit("llm:phase", "analyzing");
        let holder = guard.as_ref().expect("model is initialized");

        // ── 컨텍스트 + 토크나이즈 ─────────────────────────────────
        let ctx_params = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(N_CTX));
        let mut ctx = holder
            .model
            .new_context(&holder.backend, ctx_params)
            .map_err(|e| format!("컨텍스트 생성 실패: {e}"))?;

        let tokens = holder
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| format!("토크나이즈 실패: {e}"))?;

        let n_ctx_i = ctx.n_ctx() as i32;
        let n_kv_req = tokens.len() as i32 + n_gen;
        if n_kv_req > n_ctx_i {
            return Err(format!(
                "프롬프트가 너무 깁니다 ({}토큰). 컨텍스트 한도: {}",
                tokens.len(),
                n_ctx_i
            ));
        }

        // ── 프롬프트 배치 디코딩 ───────────────────────────────────
        let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
        let last = (tokens.len() - 1) as i32;
        for (i, t) in (0_i32..).zip(tokens.iter()) {
            batch
                .add(*t, i, &[0], i == last)
                .map_err(|e| e.to_string())?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("디코딩 실패: {e}"))?;

        // ── 샘플러 ────────────────────────────────────────────────
        // temperature == 0 → greedy (결정적)
        // temperature >  0 → 온도 스케일링 후 확률 샘플링 (랜덤 시드로 매번 다름)
        let random_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(42);

        let mut sampler = if temperature <= 0.0 {
            LlamaSampler::chain_simple([LlamaSampler::greedy()])
        } else {
            // top_k → top_p → min_p 로 확률 낮은(비논리적인) 토큰을 먼저 잘라낸 뒤
            // temp+dist 로 샘플링한다. 온도만 단독으로 쓰면 전체 분포에서 뽑기 때문에
            // 가끔 엉뚱한 토큰이 선택되어 문맥에서 벗어난 답이 나올 수 있다.
            LlamaSampler::chain_simple([
                LlamaSampler::top_k(40),
                LlamaSampler::top_p(0.9, 1),
                LlamaSampler::min_p(0.05, 1),
                LlamaSampler::temp(temperature),
                LlamaSampler::dist(random_seed),
            ])
        };

        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();
        let mut n_cur = batch.n_tokens();
        let max_pos = n_cur + n_gen;

        while n_cur < max_pos {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);

            if holder.model.is_eog_token(token) {
                break;
            }

            let piece = holder
                .model
                .token_to_piece(token, &mut decoder, true, None)
                .map_err(|e| e.to_string())?;
            output.push_str(&piece);

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| e.to_string())?;
            n_cur += 1;

            ctx.decode(&mut batch)
                .map_err(|e| format!("디코딩 실패: {e}"))?;
        }

        Ok(output.replace("<end_of_turn>", "").trim().to_string())
    })
    .await
    .map_err(|e| format!("스레드 오류: {e}"))?
}

// ── 커맨드: 종합 보안 분석 ─────────────────────────────────────

#[tauri::command]
pub async fn llm_analyze(app: tauri::AppHandle, report: ScanReport) -> Result<LlmAnalysis, String> {
    let prompt = build_analyze_prompt(&report);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 개별 항목 심층 설명 ───────────────────────────────

#[tauri::command]
pub async fn llm_explain_check(
    app: tauri::AppHandle,
    check: CheckResult,
) -> Result<LlmAnalysis, String> {
    let prompt = build_explain_prompt(&check);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 자연어 질문 (Q&A) ─────────────────────────────────

#[derive(Deserialize)]
pub struct QaTurn {
    pub question: String,
    pub answer: String,
}

#[tauri::command]
pub async fn llm_ask(
    app: tauri::AppHandle,
    report: ScanReport,
    question: String,
    #[allow(non_snake_case)] priorTurns: Option<Vec<QaTurn>>,
) -> Result<LlmAnalysis, String> {
    let prompt = build_ask_prompt(&report, &question, &priorTurns.unwrap_or_default());
    // Q&A는 질문마다 다른 답변이 나오도록 temperature 0.7 사용
    let summary = run_inference(app, prompt, 0.7).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 보안 로드맵 생성 ──────────────────────────────────

#[tauri::command]
pub async fn llm_roadmap(app: tauri::AppHandle, report: ScanReport) -> Result<LlmAnalysis, String> {
    let prompt = build_roadmap_prompt(&report);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 프로세스 이상 탐지 분석 ──────────────────────────

#[tauri::command]
pub async fn llm_analyze_processes(
    app: tauri::AppHandle,
    processes: Vec<ProcessInfo>,
) -> Result<LlmAnalysis, String> {
    let prompt = build_process_prompt(&processes);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 네트워크 연결 분석 ────────────────────────────────

#[tauri::command]
pub async fn llm_analyze_network(
    app: tauri::AppHandle,
    connections: Vec<NetworkConn>,
) -> Result<LlmAnalysis, String> {
    let prompt = build_network_prompt(&connections);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 브라우저 확장 프로그램 분석 ──────────────────────

#[tauri::command]
pub async fn llm_analyze_extensions(
    app: tauri::AppHandle,
    extensions: Vec<BrowserExtension>,
) -> Result<LlmAnalysis, String> {
    let prompt = build_extension_prompt(&extensions);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 모델 언로드 ────────────────────────────────────────

#[tauri::command]
pub fn llm_unload_model() -> Result<(), String> {
    let mut guard = MODEL.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

/// 모델이 현재 메모리에 로드되어 있는지 확인한다.
#[tauri::command]
pub fn llm_is_model_loaded() -> bool {
    MODEL.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// 모델을 메모리에서 언로드하고 파일도 삭제한다.
#[tauri::command]
pub fn llm_delete_model() -> Result<(), String> {
    // 1. 메모리에서 언로드
    {
        let mut guard = MODEL.lock().map_err(|e| e.to_string())?;
        *guard = None;
    }
    // 2. 파일 삭제
    let path = active_model_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("모델 파일 삭제 실패: {e}"))?;
    }

    // 3. 같은 폴더에 남아 있을 수 있는 옛 파일명·중단된 .part 도 함께 정리한다.
    //    (하나만 지우면 "제거"했는데 다시 설치됨으로 보이는 일이 생긴다)
    let mut dirs = vec![install_dir()];
    if let Some(parent) = path.parent() {
        if !dirs.contains(&parent.to_path_buf()) {
            dirs.push(parent.to_path_buf());
        }
    }
    for dir in dirs {
        let _ = std::fs::remove_file(dir.join(format!("{MODEL_FILENAME}.part")));
        for name in std::iter::once(&MODEL_FILENAME).chain(LEGACY_MODEL_FILENAMES.iter()) {
            let p = dir.join(name);
            if p != path && p.exists() {
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    // 4. 설정 파일의 경로도 초기화
    let sp = settings_path();
    if sp.exists() {
        let _ = std::fs::remove_file(&sp);
    }
    Ok(())
}

// ── 프롬프트 빌더 ─────────────────────────────────────────────

fn severity_label(s: &Severity) -> &'static str {
    match s {
        Severity::Ok => "정상",
        Severity::Info => "정보",
        Severity::Warning => "경고",
        Severity::Danger => "위험",
        Severity::Critical => "심각",
    }
}

/// 공통: 위험·주의 항목 목록 추출
fn danger_warning_sections(report: &ScanReport) -> (String, String) {
    let danger: Vec<String> = report
        .checks
        .iter()
        .filter(|c| matches!(c.severity, Severity::Danger | Severity::Critical))
        .map(|c| format!("- {}: {}", c.title, c.message))
        .collect();

    let warning: Vec<String> = report
        .checks
        .iter()
        .filter(|c| c.severity == Severity::Warning)
        .map(|c| format!("- {}: {}", c.title, c.message))
        .collect();

    let d = if danger.is_empty() {
        "없음".into()
    } else {
        danger.join("\n")
    };
    let w = if warning.is_empty() {
        "없음".into()
    } else {
        warning.join("\n")
    };
    (d, w)
}

/// 종합 분석 프롬프트
fn build_analyze_prompt(report: &ScanReport) -> String {
    let (danger_section, warning_section) = danger_warning_sections(report);
    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 아래 보안 검사 결과를 분석하여 \
반드시 다음 마크다운 형식으로 한국어 답변을 작성하세요.\n\n\
[보안 점수] {score}/100\n\n\
[위험 항목]\n{danger}\n\n\
[주의 항목]\n{warning}\n\n\
---\n\
아래 형식을 정확히 따르세요 (섹션 제목은 그대로 유지):\n\n\
## 🚨 즉시 조치 필요\n\
위험 항목 각각에 대해 다음 형식으로 작성:\n\
- **[항목명]**: 이 취약점이 위험한 구체적인 이유와 실제 피해 가능성\n\n\
## 🎯 공격자 악용 시나리오\n\
위 취약점들을 실제 공격자가 조합·악용할 경우 발생할 수 있는 구체적인 공격 흐름을 \
2~3문장으로 서술하세요.\n\n\
## ✅ 우선순위별 조치 방법\n\
가장 중요한 조치부터 번호를 매겨 나열:\n\
1. **[조치명]**: 실행 방법 (설정 경로 또는 명령어 포함)\n\
2. **[조치명]**: 실행 방법\n\
3. **[조치명]**: 실행 방법\n\
(필요 시 더 추가)\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        score = report.score,
        danger = danger_section,
        warning = warning_section,
    )
}

/// 개별 항목 설명 프롬프트
fn build_explain_prompt(check: &CheckResult) -> String {
    let evidence_str = if check.evidence.is_empty() {
        "없음".to_string()
    } else {
        check
            .evidence
            .iter()
            .map(|(k, v)| format!("  {k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 아래 보안 점검 항목을 심층 분석하여 \
마크다운 형식으로 한국어 답변을 작성하세요.\n\n\
[점검 항목] {title}\n\
[심각도] {severity}\n\
[검출 내용] {message}\n\
[기술 증거]\n{evidence}\n\n\
---\n\
반드시 다음 형식을 따르세요:\n\n\
## 🔍 취약점 상세\n\
이 항목이 위험한 이유와 공격자가 어떻게 악용할 수 있는지 설명하세요.\n\n\
## 🎯 실제 공격 시나리오\n\
이 취약점을 이용한 구체적인 공격 흐름을 2~3문장으로 서술하세요.\n\n\
## ✅ 단계별 조치 방법\n\
1. **[단계명]**: 구체적인 실행 방법 (설정 경로 또는 명령어 포함)\n\
2. **[단계명]**: 구체적인 실행 방법\n\
3. **[단계명]**: 구체적인 실행 방법\n\n\
## 💡 추가 권장 사항\n\
관련 보안 강화 방법이나 모니터링 팁을 간략히 서술하세요.\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        title = check.title,
        severity = severity_label(&check.severity),
        message = check.message,
        evidence = evidence_str,
    )
}

/// Q&A 답변에서 후속 대화에 다시 넣을 때는 길이를 제한해 컨텍스트 폭주를 막는다.
const QA_HISTORY_ANSWER_MAX: usize = 600;
/// 프롬프트에 포함할 직전 대화 최대 턴 수(질문+답변 쌍).
const QA_HISTORY_MAX_TURNS: usize = 4;

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}

/// 자연어 질문 프롬프트.
/// 첫 user 턴에 시스템 지시 + 보안 상태를 넣고, 이후 실제 대화 이력을
/// Gemma chat 형식(user/model 교대)으로 이어 붙여 멀티턴 맥락을 유지한다.
fn build_ask_prompt(report: &ScanReport, question: &str, prior_turns: &[QaTurn]) -> String {
    let danger_count = report
        .checks
        .iter()
        .filter(|c| matches!(c.severity, Severity::Danger | Severity::Critical))
        .count();
    let warning_count = report
        .checks
        .iter()
        .filter(|c| c.severity == Severity::Warning)
        .count();

    let top_issues: Vec<String> = report
        .checks
        .iter()
        .filter(|c| {
            matches!(
                c.severity,
                Severity::Danger | Severity::Critical | Severity::Warning
            )
        })
        .take(8)
        .map(|c| {
            format!(
                "- [{}] {}: {}",
                severity_label(&c.severity),
                c.title,
                c.message
            )
        })
        .collect();

    let top_issues_str = if top_issues.is_empty() {
        "없음".into()
    } else {
        top_issues.join("\n")
    };

    let mut prompt = String::new();

    // ── 첫 user 턴: 시스템 지시 + 현재 보안 상태 (한 번만) ──
    prompt.push_str(&format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 어시스턴트입니다. 사용자의 Windows 보안에 관한 질문에 \
정확하고 실용적인 한국어로 답하세요.\n\
- 아래 [현재 시스템 보안 상태]에 근거해 답하고, 확실하지 않으면 추측하지 마세요.\n\
- 답은 마크다운으로, 핵심만 간결하게. 불필요한 서론·반복 금지.\n\
- 실행 방법을 물으면 설정 경로나 명령어를 구체적으로 제시하세요.\n\
- 이전 대화 맥락이 있으면 이어서 답하세요.\n\n\
[현재 시스템 보안 상태]\n\
- 보안 점수: {score}/100\n\
- 위험 항목 {danger_count}개, 주의 항목 {warning_count}개\n\
{top_issues}\n\
<end_of_turn>\n\
<start_of_turn>model\n\
네, WinGuard 보안 어시스턴트입니다. 무엇을 도와드릴까요?<end_of_turn>\n",
        score = report.score,
        danger_count = danger_count,
        warning_count = warning_count,
        top_issues = top_issues_str,
    ));

    // ── 직전 대화 이력 (최근 QA_HISTORY_MAX_TURNS 턴) ──
    let start = prior_turns.len().saturating_sub(QA_HISTORY_MAX_TURNS);
    for turn in &prior_turns[start..] {
        prompt.push_str(&format!(
            "<start_of_turn>user\n{q}<end_of_turn>\n\
<start_of_turn>model\n{a}<end_of_turn>\n",
            q = turn.question.trim(),
            a = truncate_chars(turn.answer.trim(), QA_HISTORY_ANSWER_MAX),
        ));
    }

    // ── 이번 질문 ──
    prompt.push_str(&format!(
        "<start_of_turn>user\n{question}<end_of_turn>\n<start_of_turn>model\n",
        question = question.trim(),
    ));

    prompt
}

/// 프로세스 분석 프롬프트
fn build_process_prompt(processes: &[ProcessInfo]) -> String {
    let unsigned: Vec<String> = processes
        .iter()
        .filter(|p| p.signed == Some(false))
        .map(|p| {
            format!(
                "- {} (PID {}) | CPU {:.1}% | {} | {}",
                p.name,
                p.pid,
                p.cpu,
                p.company.as_deref().unwrap_or("회사 정보 없음"),
                p.path.as_deref().unwrap_or("경로 없음"),
            )
        })
        .collect();

    let high_cpu: Vec<String> = processes
        .iter()
        .filter(|p| p.cpu > 5.0)
        .take(10)
        .map(|p| {
            format!(
                "- {} (PID {}) | CPU {:.1}% | {}MB | 서명:{}",
                p.name,
                p.pid,
                p.cpu,
                p.memory_mb as u64,
                match p.signed {
                    Some(true) => "유효",
                    Some(false) => "없음/무효",
                    None => "확인불가",
                }
            )
        })
        .collect();

    let unsigned_str = if unsigned.is_empty() {
        "없음 (모든 프로세스 서명 확인됨)".into()
    } else {
        unsigned.join("\n")
    };

    let cpu_str = if high_cpu.is_empty() {
        "없음".into()
    } else {
        high_cpu.join("\n")
    };

    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 현재 실행 중인 프로세스를 분석하여 \
이상 징후를 찾아내고 마크다운 형식으로 한국어 보고서를 작성하세요.\n\n\
[참고] \"winguard.exe\"는 현재 실행 중인 WinGuard 보안 프로그램 자체이므로 분석 대상에서 제외하세요.\n\n\
[서명 없는 프로세스]\n{unsigned}\n\n\
[CPU 사용량 높은 프로세스]\n{cpu}\n\n\
---\n\
반드시 다음 형식을 따르세요:\n\n\
## 🚨 의심 프로세스\n\
서명 없거나 비정상적으로 CPU를 많이 사용하는 위험 프로세스를 나열하세요:\n\
- **[프로세스명]**: 위험한 이유와 악성코드 가능성\n\n\
## ✅ 현재 상태 요약\n\
전체 프로세스 보안 상태를 2~3문장으로 요약하세요.\n\n\
## 💡 권고 조치\n\
의심 프로세스에 대한 구체적인 확인·조치 방법을 나열하세요:\n\
1. **[단계]**: 방법\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        unsigned = unsigned_str,
        cpu      = cpu_str,
    )
}

/// 네트워크 분석 프롬프트
fn build_network_prompt(connections: &[NetworkConn]) -> String {
    let established: Vec<String> = connections
        .iter()
        .filter(|c| {
            c.state == "Established"
                && !c.remote_address.starts_with("127.")
                && c.remote_address != "::1"
        })
        .take(20)
        .map(|c| {
            format!(
                "- {}:{} → {}:{} | {}",
                c.local_address,
                c.local_port,
                c.remote_address,
                c.remote_port,
                c.process_name.as_deref().unwrap_or("알 수 없음"),
            )
        })
        .collect();

    let listening: Vec<String> = connections
        .iter()
        .filter(|c| c.state == "Listen")
        .take(15)
        .map(|c| {
            format!(
                "- 포트 {} | {}",
                c.local_port,
                c.process_name.as_deref().unwrap_or("알 수 없음"),
            )
        })
        .collect();

    let est_str = if established.is_empty() {
        "외부 연결 없음".into()
    } else {
        established.join("\n")
    };

    let listen_str = if listening.is_empty() {
        "없음".into()
    } else {
        listening.join("\n")
    };

    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 현재 TCP 연결 상태를 분석하여 \
이상 징후를 찾아내고 마크다운 형식으로 한국어 보고서를 작성하세요.\n\n\
[외부 연결 (ESTABLISHED)]\n{established}\n\n\
[열린 포트 (LISTEN)]\n{listening}\n\n\
---\n\
반드시 다음 형식을 따르세요:\n\n\
## 🚨 의심 연결\n\
비정상적이거나 위험한 연결을 나열하세요 (없으면 '없음' 기재):\n\
- **[연결]**: 위험한 이유\n\n\
## 🔓 노출된 포트 위험도\n\
외부에 열린 주요 포트와 위험도를 나열하세요:\n\
- **포트 [번호]**: 서비스명, 위험 수준, 권고 사항\n\n\
## ✅ 전체 네트워크 상태\n\
현재 네트워크 보안 상태를 2~3문장으로 요약하세요.\n\n\
## 💡 권고 조치\n\
1. **[단계]**: 방법\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        established = est_str,
        listening = listen_str,
    )
}

/// 브라우저 확장 프로그램 분석 프롬프트
fn build_extension_prompt(extensions: &[BrowserExtension]) -> String {
    // 고위험 권한: 트래픽 가로채기·네이티브 통신·디버거·전체 URL 접근
    const HIGH_RISK: &[&str] = &[
        "webrequest",
        "webrequestblocking",
        "nativemessaging",
        "debugger",
        "<all_urls>",
    ];
    // 중위험 권한: 방문 기록·쿠키·다운로드·클립보드 등
    const MED_RISK: &[&str] = &[
        "history",
        "cookies",
        "downloads",
        "management",
        "clipboardread",
        "clipboardwrite",
        "http://*/*",
        "https://*/*",
    ];

    let mut high: Vec<String> = vec![];
    let mut med: Vec<String> = vec![];

    for ext in extensions {
        let p = ext.permissions.to_lowercase();
        let line = format!(
            "- {} [{}] v{} | 권한: {}",
            ext.name, ext.browser, ext.version, ext.permissions
        );
        if HIGH_RISK.iter().any(|r| p.contains(r)) {
            high.push(line);
        } else if MED_RISK.iter().any(|r| p.contains(r)) {
            med.push(line);
        }
    }

    let browsers: std::collections::HashSet<&str> =
        extensions.iter().map(|e| e.browser.as_str()).collect();
    let browser_list = browsers.into_iter().collect::<Vec<_>>().join(", ");

    let high_str = if high.is_empty() {
        "없음".into()
    } else {
        high.join("\n")
    };
    let med_str = if med.is_empty() {
        "없음".into()
    } else {
        med.join("\n")
    };

    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 설치된 확장 프로그램을 분석하여 \
위험한 확장을 식별하고 마크다운 형식으로 한국어 보고서를 작성하세요.\n\n\
[스캔 대상 브라우저] {browsers}\n\
[전체 확장 프로그램 수] {total}개\n\n\
[고위험 권한 확장 (webRequest / nativeMessaging / debugger / <all_urls>)]\n{high}\n\n\
[중위험 권한 확장 (history / cookies / downloads / clipboard)]\n{med}\n\n\
---\n\
반드시 다음 형식을 따르세요:\n\n\
## 🚨 위험 확장 프로그램\n\
실제로 위험하거나 의심스러운 확장을 나열하세요 (없으면 '없음' 기재):\n\
- **[이름]**: 위험한 이유와 악용 가능성\n\n\
## ⚠️ 주의가 필요한 확장\n\
중위험 권한을 가졌지만 정상 확장일 수도 있는 항목:\n\
- **[이름]**: 권한이 필요한 이유와 확인 방법\n\n\
## ✅ 전체 상태 요약\n\
브라우저 확장 보안 상태를 2~3문장으로 요약하세요.\n\n\
## 💡 권고 사항\n\
불필요한 확장 제거, 권한 검토 방법 등 실용적인 조치를 나열하세요.\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        browsers = browser_list,
        total = extensions.len(),
        high = high_str,
        med = med_str,
    )
}

// ── 커맨드: 이벤트 로그 분석 ──────────────────────────────────

#[tauri::command]
pub async fn llm_analyze_eventlog(
    app: tauri::AppHandle,
    entries: Vec<EventLogEntry>,
) -> Result<LlmAnalysis, String> {
    let prompt = build_eventlog_prompt(&entries);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

// ── 커맨드: 종합 보안 보고서 생성 ─────────────────────────────

#[tauri::command]
pub async fn llm_generate_report(
    app: tauri::AppHandle,
    report: ScanReport,
) -> Result<LlmAnalysis, String> {
    let prompt = build_full_report_prompt(&report);
    let summary = run_inference_n(app, prompt, 0.0, N_GENERATE_REPORT).await?;
    Ok(LlmAnalysis { summary })
}

/// 보안 로드맵 프롬프트
fn build_roadmap_prompt(report: &ScanReport) -> String {
    let (danger_section, warning_section) = danger_warning_sections(report);
    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 아래 보안 검사 결과를 바탕으로 \
단계별 보안 강화 로드맵을 마크다운 형식으로 작성하세요.\n\n\
[보안 점수] {score}/100\n\n\
[위험 항목]\n{danger}\n\n\
[주의 항목]\n{warning}\n\n\
---\n\
반드시 다음 형식을 따르세요:\n\n\
## 🚨 즉시 조치 (이번 주)\n\
가장 긴급한 조치 2~3가지를 나열하세요:\n\
- **[조치명]**: 방법과 예상 소요 시간\n\n\
## 📅 단기 개선 (이번 달)\n\
중요하지만 즉각적이지 않은 보안 강화 항목을 나열하세요:\n\
- **[항목명]**: 방법과 기대 효과\n\n\
## 🛡️ 장기 강화 (3개월 이내)\n\
지속적인 보안 유지를 위한 정책·습관을 제안하세요:\n\
- **[제안명]**: 구체적인 실행 방법\n\n\
## 📊 개선 후 예상 보안 점수\n\
위 조치를 모두 실행했을 때 예상되는 점수와 그 근거를 한 문장으로 작성하세요.\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        score = report.score,
        danger = danger_section,
        warning = warning_section,
    )
}

/// 이벤트 로그 분석 프롬프트
fn build_eventlog_prompt(entries: &[EventLogEntry]) -> String {
    let mut high: Vec<String> = vec![];
    let mut warn: Vec<String> = vec![];

    for e in entries {
        let line = format!(
            "- [{}] {} | {} | {}",
            e.time_created, e.label, e.source, e.message
        );
        if e.level == "고위험" {
            high.push(line);
        } else if e.level == "주의" {
            warn.push(line);
        }
    }

    // 이벤트별 통계
    let fail_login = entries.iter().filter(|e| e.event_id == 4625).count();
    let lockout = entries.iter().filter(|e| e.event_id == 4740).count();
    let new_account = entries.iter().filter(|e| e.event_id == 4720).count();
    let new_svc = entries
        .iter()
        .filter(|e| e.event_id == 4697 || e.event_id == 7045)
        .count();
    let new_task = entries.iter().filter(|e| e.event_id == 4698).count();

    let high_str = if high.is_empty() {
        "없음".into()
    } else {
        high.join("\n")
    };
    let warn_str = if warn.is_empty() {
        "없음".into()
    } else {
        warn.join("\n")
    };

    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 최근 24시간 보안 이벤트 로그를 분석하여 \
이상 징후와 침해 지표를 찾아 마크다운 한국어 보고서를 작성하세요.\n\n\
[이벤트 통계]\n\
- 총 이벤트: {total}건\n\
- 로그인 실패 (4625): {fail_login}건\n\
- 계정 잠금 (4740): {lockout}건\n\
- 계정 생성 (4720): {new_account}건\n\
- 서비스 설치 (4697/7045): {new_svc}건\n\
- 예약 작업 생성 (4698): {new_task}건\n\n\
[고위험 이벤트]\n{high}\n\n\
[주의 이벤트]\n{warn}\n\n\
---\n\
아래 4개 섹션을 순서대로 작성하세요:\n\n\
## 🚨 이상 징후 탐지\n\
- **[패턴명]**: 의미와 위협 수준 (없으면 - 없음)\n\n\
## 📊 이벤트 패턴 분석\n\
(통계 기반 공격 흔적을 2~3문장으로)\n\n\
## ✅ 현재 상태 평가\n\
(전반적인 보안 활동 상태를 1~2문장으로)\n\n\
## 💡 권고 조치\n\
1. **[조치명]**: 구체적인 방법\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        total = entries.len(),
        fail_login = fail_login,
        lockout = lockout,
        new_account = new_account,
        new_svc = new_svc,
        new_task = new_task,
        high = high_str,
        warn = warn_str,
    )
}

/// 종합 보안 보고서 프롬프트 (경영진/비전문가 대상)
fn build_full_report_prompt(report: &ScanReport) -> String {
    let dt = {
        use std::time::{Duration, UNIX_EPOCH};
        let secs = report.scanned_at;
        // 날짜 포맷은 단순히 초 → 문자열 (정확한 날짜 계산은 chrono 없이 근사)
        let _d = UNIX_EPOCH + Duration::from_secs(secs as u64);
        format!("{}", secs) // 프롬프트에서 타임스탬프로 충분
    };

    let ok_count = report
        .checks
        .iter()
        .filter(|c| matches!(c.severity, Severity::Ok))
        .count();
    let warning_count = report
        .checks
        .iter()
        .filter(|c| c.severity == Severity::Warning)
        .count();
    let danger_count = report
        .checks
        .iter()
        .filter(|c| matches!(c.severity, Severity::Danger | Severity::Critical))
        .count();

    // 전체 항목 목록 (심각도 이모지 포함)
    let all_checks: Vec<String> = report
        .checks
        .iter()
        .map(|c| {
            let icon = match c.severity {
                Severity::Ok => "✅",
                Severity::Info => "ℹ️",
                Severity::Warning => "⚠️",
                Severity::Danger => "🔴",
                Severity::Critical => "🚨",
            };
            format!("{} {} — {}", icon, c.title, c.message)
        })
        .collect();

    let (danger_section, warning_section) = danger_warning_sections(report);

    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 아래 보안 점검 결과를 바탕으로 \
최고경영진·비전문가도 이해할 수 있는 공식 보안 보고서를 마크다운 형식으로 한국어로 작성하세요.\n\n\
[점검 일시] Unix 타임스탬프: {dt}\n\
[보안 점수] {score}/100\n\
[항목 현황] 정상 {ok}개 · 주의 {warn}개 · 위험+심각 {danger}개\n\n\
[위험·심각 항목]\n{danger_items}\n\n\
[주의 항목]\n{warning_items}\n\n\
[전체 점검 항목]\n{all_checks}\n\n\
---\n\
반드시 다음 구조로 보고서를 작성하세요:\n\n\
# 📊 WinGuard 보안 점검 보고서\n\n\
## 1. 요약\n\
현재 보안 점수의 의미, 위험 수준, 시급성을 3~4문장으로 서술하세요.\n\n\
## 2. 즉시 조치 필요 사항\n\
위험·심각 항목 각각에 대해:\n\
- **[항목명]**: 문제 내용, 위험성, 권고 조치\n\n\
## 3. 주의 사항\n\
주의 항목 각각에 대해:\n\
- **[항목명]**: 내용과 개선 방향\n\n\
## 4. 보안 강화 로드맵\n\
### 단기 (이번 주)\n\
### 중기 (이번 달)\n\
### 장기 (3개월 이내)\n\n\
## 5. 결론\n\
전체 평가와 개선 후 예상 보안 점수를 2문장으로 작성하세요.\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        dt = dt,
        score = report.score,
        ok = ok_count,
        warn = warning_count,
        danger = danger_count,
        danger_items = danger_section,
        warning_items = warning_section,
        all_checks = all_checks.join("\n"),
    )
}

// ── 커맨드: 장치(카메라·마이크) 보안 분석 ─────────────────────────

#[tauri::command]
pub async fn llm_analyze_device(
    app: tauri::AppHandle,
    status: DeviceStatus,
) -> Result<LlmAnalysis, String> {
    let prompt = build_device_prompt(&status);
    let summary = run_inference(app, prompt, 0.0).await?;
    Ok(LlmAnalysis { summary })
}

/// 장치 접근 제어 분석 프롬프트
fn build_device_prompt(status: &DeviceStatus) -> String {
    // 위험 앱 분류 (Win32 앱 중 위험 경로 또는 의심 런타임)
    const RISKY_NAMES: &[&str] = &[
        "python.exe",
        "pythonw.exe",
        "node.exe",
        "powershell.exe",
        "wscript.exe",
        "cscript.exe",
        "mshta.exe",
        "cmd.exe",
        "regsvr32.exe",
        "rundll32.exe",
    ];
    const RISKY_PATHS: &[&str] = &["downloads", "temp", "tmp", "appdata\\local\\temp"];
    const SAFE_PATHS: &[&str] = &[
        "system32",
        "program files",
        "windowsapps",
        "microsoft",
        "windows defender",
    ];

    let classify = |app: &AppAccess| -> &'static str {
        if app.is_uwp {
            return "정상";
        }
        let lower = app.path.to_lowercase();
        let name = app.name.to_lowercase();
        if RISKY_NAMES.iter().any(|r| name == *r) || RISKY_PATHS.iter().any(|r| lower.contains(r)) {
            "위험"
        } else if SAFE_PATHS.iter().any(|r| lower.contains(r)) {
            "정상"
        } else {
            "주의"
        }
    };

    let fmt_apps = |apps: &[AppAccess]| -> String {
        if apps.is_empty() {
            return "등록된 앱 없음".into();
        }
        apps.iter()
            .map(|a| {
                let state = if a.allowed { "허용" } else { "차단" };
                let risk = classify(a);
                let kind = if a.is_uwp { "UWP" } else { "Win32" };
                if a.path.is_empty() {
                    format!("- [{}][{}][{}] {}", risk, state, kind, a.name)
                } else {
                    format!("- [{}][{}][{}] {} ({})", risk, state, kind, a.name, a.path)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let cam_global = if status.camera_allowed {
        "허용"
    } else {
        "차단"
    };
    let mic_global = if status.mic_allowed {
        "허용"
    } else {
        "차단"
    };

    let risky_cam: Vec<&AppAccess> = status
        .camera_apps
        .iter()
        .filter(|a| classify(a) == "위험" && a.allowed)
        .collect();
    let risky_mic: Vec<&AppAccess> = status
        .mic_apps
        .iter()
        .filter(|a| classify(a) == "위험" && a.allowed)
        .collect();

    let risky_cam_str = if risky_cam.is_empty() {
        "없음".into()
    } else {
        risky_cam
            .iter()
            .map(|a| format!("- {}", a.name))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let risky_mic_str = if risky_mic.is_empty() {
        "없음".into()
    } else {
        risky_mic
            .iter()
            .map(|a| format!("- {}", a.name))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "<start_of_turn>user\n\
당신은 WinGuard 보안 분석 시스템입니다. 아래 카메라·마이크 접근 권한 현황을 분석하여 \
프라이버시 위협을 파악하고 한국어 마크다운 보고서를 작성하세요.\n\n\
카메라 전역 설정: {cam_global}\n\
마이크 전역 설정: {mic_global}\n\n\
카메라 접근 앱 목록:\n{cam_apps}\n\n\
마이크 접근 앱 목록:\n{mic_apps}\n\n\
위험 앱 (카메라 허용): {risky_cam}\n\
위험 앱 (마이크 허용): {risky_mic}\n\n\
---\n\
다음 4개 섹션을 순서대로 작성하세요. 각 섹션은 위 데이터를 기반으로 실제 내용을 채워야 합니다.\n\n\
## 🚨 위험 징후\n\
위험 앱 각각에 대해 앱 이름과 프라이버시 침해 가능성을 설명하세요. 위험 앱이 없으면 \"현재 탐지된 위험 징후 없음\"이라고 쓰세요.\n\n\
## 📊 접근 권한 분석\n\
카메라와 마이크 허용 현황 및 위험 수준을 2~3문장으로 분석하세요.\n\n\
## ✅ 현재 상태 평가\n\
전반적인 프라이버시 보호 수준을 1~2문장으로 평가하세요.\n\n\
## 💡 권고 조치\n\
위험을 줄이기 위한 구체적인 조치를 번호 목록으로 작성하세요.\n\
<end_of_turn>\n\
<start_of_turn>model\n",
        cam_global  = cam_global,
        mic_global  = mic_global,
        cam_apps    = fmt_apps(&status.camera_apps),
        mic_apps    = fmt_apps(&status.mic_apps),
        risky_cam   = risky_cam_str,
        risky_mic   = risky_mic_str,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn empty_report() -> ScanReport {
        ScanReport {
            score: 80,
            checks: vec![],
            scanned_at: 0,
        }
    }

    fn turn(q: &str, a: &str) -> QaTurn {
        QaTurn {
            question: q.into(),
            answer: a.into(),
        }
    }

    #[test]
    fn local_model_filename_is_winguard_owned() {
        // 업스트림 파일명을 그대로 쓰지 않는다 — 배포처가 이름을 바꿔도 로컬 경로는 그대로.
        assert!(MODEL_FILENAME.starts_with("winguard-"));
        assert!(MODEL_FILENAME.ends_with(".gguf"));
        assert!(!MODEL_URL.ends_with(MODEL_FILENAME));
    }

    #[test]
    fn legacy_filenames_cover_previous_upstream_name() {
        // 구버전이 저장해 둔 파일을 다시 받지 않도록 URL 의 파일명이 목록에 있어야 한다.
        let upstream = MODEL_URL
            .rsplit('/')
            .next()
            .expect("URL 에는 파일명이 있다");
        assert!(LEGACY_MODEL_FILENAMES.contains(&upstream));
        assert!(!LEGACY_MODEL_FILENAMES.contains(&MODEL_FILENAME));
    }

    #[test]
    fn model_info_exposes_metadata_for_ui() {
        let info = llm_model_info();
        assert_eq!(info.file_name, MODEL_FILENAME);
        assert_eq!(info.url, MODEL_URL);
        assert!(info.size_bytes > 0);
        assert!(!info.display_name.is_empty());
        assert!(!info.default_dir.is_empty());
    }

    #[test]
    fn truncate_chars_respects_char_boundary() {
        // 한글(멀티바이트)에서도 char 단위로 자르고 패닉하지 않아야 한다.
        let s = "가나다라마";
        assert_eq!(truncate_chars(s, 3), "가나다…");
        assert_eq!(truncate_chars(s, 10), "가나다라마"); // 길이 이하면 그대로
    }

    #[test]
    fn ask_prompt_without_history_has_single_final_model_turn() {
        let p = build_ask_prompt(&empty_report(), "UAC가 뭔가요?", &[]);
        // 이번 질문이 마지막 user 턴으로, 그 뒤에 model 응답 유도 태그로 끝나야 한다.
        assert!(
            p.ends_with("<start_of_turn>user\nUAC가 뭔가요?<end_of_turn>\n<start_of_turn>model\n")
        );
        // 첫 시스템 턴은 한 번만 등장한다.
        assert_eq!(p.matches("WinGuard 보안 분석 어시스턴트").count(), 1);
    }

    #[test]
    fn ask_prompt_includes_prior_turns_in_order() {
        let prior = vec![turn("첫 질문", "첫 답변"), turn("둘째 질문", "둘째 답변")];
        let p = build_ask_prompt(&empty_report(), "셋째 질문", &prior);
        // 이전 대화의 질문·답변이 모두 프롬프트에 포함되어야 한다(멀티턴 맥락).
        assert!(p.contains("첫 질문"));
        assert!(p.contains("첫 답변"));
        assert!(p.contains("둘째 질문"));
        assert!(p.contains("셋째 질문"));
        // 순서: 첫 질문이 둘째 질문보다 앞에 온다.
        assert!(p.find("첫 질문").unwrap() < p.find("둘째 질문").unwrap());
        assert!(p.find("둘째 질문").unwrap() < p.find("셋째 질문").unwrap());
    }

    #[test]
    fn ask_prompt_caps_prior_turns_to_recent_window() {
        // QA_HISTORY_MAX_TURNS 를 초과하면 가장 오래된 턴은 제외된다.
        let prior: Vec<QaTurn> = (0..(QA_HISTORY_MAX_TURNS + 2))
            .map(|i| turn(&format!("질문{i}"), &format!("답변{i}")))
            .collect();
        let p = build_ask_prompt(&empty_report(), "최신 질문", &prior);
        assert!(!p.contains("질문0")); // 가장 오래된 것은 잘림
        assert!(!p.contains("질문1"));
        assert!(p.contains(&format!("질문{}", QA_HISTORY_MAX_TURNS + 1))); // 최근 것은 포함
    }

    #[test]
    fn ask_prompt_truncates_long_prior_answer() {
        let long_answer = "가".repeat(QA_HISTORY_ANSWER_MAX + 100);
        let prior = vec![turn("긴 답변 질문", &long_answer)];
        let p = build_ask_prompt(&empty_report(), "다음 질문", &prior);
        // 원본 전체 길이가 그대로 들어가면 안 되고, 말줄임표가 붙어야 한다.
        assert!(!p.contains(&long_answer));
        assert!(p.contains('…'));
    }

    #[test]
    fn ask_prompt_reflects_report_severity_counts() {
        let report = ScanReport {
            score: 55,
            checks: vec![
                CheckResult {
                    id: "a".into(),
                    title: "위험항목".into(),
                    severity: Severity::Danger,
                    message: "m".into(),
                    action_uri: None,
                    evidence: HashMap::new(),
                    checked_at: 0,
                },
                CheckResult {
                    id: "b".into(),
                    title: "주의항목".into(),
                    severity: Severity::Warning,
                    message: "m".into(),
                    action_uri: None,
                    evidence: HashMap::new(),
                    checked_at: 0,
                },
            ],
            scanned_at: 0,
        };
        let p = build_ask_prompt(&report, "질문", &[]);
        assert!(p.contains("55/100"));
        assert!(p.contains("위험 항목 1개, 주의 항목 1개"));
        assert!(p.contains("위험항목"));
    }
}
