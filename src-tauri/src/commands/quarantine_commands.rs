// src-tauri/src/commands/quarantine_commands.rs
//
// Fix 실행 전 레지스트리 원본 값을 백업하고, 사용자가 원클릭으로 복원할 수 있게 하는
// 격리(Quarantine) 시스템.
//
// 저장 위치: %LOCALAPPDATA%\WinGuard\quarantine\{timestamp_ms}\
//   ├── metadata.json
//   └── registry.json

use crate::util::registry::{read_dword, read_dword_hkcu};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 격리 폴더 개수가 이 값을 초과하면 quarantine_list 호출 시 오래된 항목부터
/// PRUNE_BATCH 개를 자동 삭제한다 (§4-11).
const PRUNE_THRESHOLD: usize = 50;
const PRUNE_BATCH: usize = 10;

// ── 데이터 구조 ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegBackupEntry {
    pub hive: String, // "HKLM" | "HKCU"
    pub path: String,
    pub name: String,
    pub value_type: String, // "DWORD" | "SZ"
    pub original_value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineMetadata {
    pub id: String,
    pub check_id: String,
    pub action_label: String,
    pub created_at: String,
    pub restored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineRecord {
    pub metadata: QuarantineMetadata,
    pub entries: Vec<RegBackupEntry>,
}

// ── 내부 헬퍼 ────────────────────────────────────────────────────

fn quarantine_dir() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir().ok_or("LOCALAPPDATA 경로를 찾을 수 없습니다")?;
    Ok(base.join("WinGuard").join("quarantine"))
}

fn read_record(dir: &Path) -> Option<QuarantineRecord> {
    let meta_raw = std::fs::read_to_string(dir.join("metadata.json")).ok()?;
    let metadata: QuarantineMetadata = serde_json::from_str(&meta_raw).ok()?;
    let reg_raw = std::fs::read_to_string(dir.join("registry.json")).ok()?;
    let entries: Vec<RegBackupEntry> = serde_json::from_str(&reg_raw).ok()?;
    Some(QuarantineRecord { metadata, entries })
}

/// 격리 폴더가 PRUNE_THRESHOLD 를 초과하면 오래된 항목부터 PRUNE_BATCH 개를 삭제한다.
/// restored=true 인 항목을 우선 삭제 대상으로 삼는다 (이미 복원되어 보존 가치가 낮음).
fn prune_if_needed(dir: &Path, records: &mut Vec<QuarantineRecord>) {
    if records.len() <= PRUNE_THRESHOLD {
        return;
    }

    let mut candidates: Vec<usize> = (0..records.len()).collect();
    candidates.sort_by(|&a, &b| {
        let ra = &records[a].metadata;
        let rb = &records[b].metadata;
        // restored=true 를 앞으로 (삭제 우선), 그 다음 id(타임스탬프 문자열) 오름차순(오래된 것 우선)
        rb.restored
            .cmp(&ra.restored)
            .then_with(|| ra.id.cmp(&rb.id))
    });

    let to_delete: Vec<String> = candidates
        .into_iter()
        .take(PRUNE_BATCH)
        .map(|i| records[i].metadata.id.clone())
        .collect();

    for id in &to_delete {
        let _ = std::fs::remove_dir_all(dir.join(id));
    }

    records.retain(|r| !to_delete.contains(&r.metadata.id));
}

// ── Tauri 커맨드 ──────────────────────────────────────────────────

/// 프론트엔드가 Fix 실행 전 원본 DWORD 값을 읽어 BACKUP_MAP 의
/// `original_value` 를 채우는 데 사용한다.
#[tauri::command]
pub fn read_reg_dword(hive: String, path: String, name: String) -> Result<u32, String> {
    match hive.as_str() {
        "HKLM" => read_dword(&path, &name),
        "HKCU" => read_dword_hkcu(&path, &name),
        other => Err(format!("알 수 없는 하이브: {}", other)),
    }
}

/// Fix 실행 직전 레지스트리 원본 값을 백업한다. 성공 시 quarantine_id 반환.
#[tauri::command]
pub fn quarantine_save(
    entries: Vec<RegBackupEntry>,
    check_id: String,
    action_label: String,
) -> Result<String, String> {
    let id = chrono::Utc::now().timestamp_millis().to_string();
    let dir = quarantine_dir()?.join(&id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("격리 디렉터리 생성 실패: {}", e))?;

    let metadata = QuarantineMetadata {
        id: id.clone(),
        check_id,
        action_label,
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        restored: false,
    };

    let meta_path = dir.join("metadata.json");
    let reg_path = dir.join("registry.json");

    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("메타데이터 직렬화 실패: {}", e))?,
    )
    .map_err(|e| format!("메타데이터 저장 실패: {}", e))?;

    std::fs::write(
        &reg_path,
        serde_json::to_string_pretty(&entries)
            .map_err(|e| format!("레지스트리 백업 직렬화 실패: {}", e))?,
    )
    .map_err(|e| format!("레지스트리 백업 저장 실패: {}", e))?;

    Ok(id)
}

/// 격리 기록 전체 조회 (최신순). 읽기 실패한 개별 항목은 건너뛴다.
#[tauri::command]
pub fn quarantine_list() -> Vec<QuarantineRecord> {
    let dir = match quarantine_dir() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(), // 폴더가 아직 없으면 빈 목록
    };

    let mut records: Vec<QuarantineRecord> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| read_record(&e.path()))
        .collect();

    prune_if_needed(&dir, &mut records);

    // 최신순 정렬 (id = 밀리초 타임스탬프 문자열, 동일 자릿수이므로 문자열 내림차순 = 최신순)
    records.sort_by(|a, b| b.metadata.id.cmp(&a.metadata.id));
    records
}

/// 선택한 격리 항목의 레지스트리 값을 원본으로 복원한다.
#[tauri::command]
pub fn quarantine_restore(quarantine_id: String) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let dir = quarantine_dir()?.join(&quarantine_id);
    let reg_path = dir.join("registry.json");

    let raw =
        std::fs::read_to_string(&reg_path).map_err(|e| format!("백업 파일 읽기 실패: {}", e))?;
    let entries: Vec<RegBackupEntry> =
        serde_json::from_str(&raw).map_err(|e| format!("백업 파일 파싱 실패: {}", e))?;

    for entry in &entries {
        let root = match entry.hive.as_str() {
            "HKLM" => RegKey::predef(HKEY_LOCAL_MACHINE),
            "HKCU" => RegKey::predef(HKEY_CURRENT_USER),
            other => return Err(format!("알 수 없는 하이브: {}", other)),
        };

        let key = root
            .open_subkey_with_flags(&entry.path, KEY_WRITE)
            .map_err(|e| format!("레지스트리 키 열기 실패: {} — {}", entry.path, e))?;

        match entry.value_type.as_str() {
            "DWORD" => {
                let v = entry.original_value.as_u64().ok_or("DWORD 값 파싱 실패")? as u32;
                key.set_value(&entry.name, &v)
                    .map_err(|e| format!("DWORD 복원 실패: {}", e))?;
            }
            "SZ" => {
                let v = entry.original_value.as_str().ok_or("SZ 값 파싱 실패")?;
                key.set_value(&entry.name, &v)
                    .map_err(|e| format!("SZ 복원 실패: {}", e))?;
            }
            other => return Err(format!("지원하지 않는 값 형식: {}", other)),
        }
    }

    // metadata.json 의 restored 플래그 갱신
    let meta_path = dir.join("metadata.json");
    let meta_raw =
        std::fs::read_to_string(&meta_path).map_err(|e| format!("메타데이터 읽기 실패: {}", e))?;
    let mut meta: QuarantineMetadata =
        serde_json::from_str(&meta_raw).map_err(|e| format!("메타데이터 파싱 실패: {}", e))?;
    meta.restored = true;
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("메타데이터 직렬화 실패: {}", e))?,
    )
    .map_err(|e| format!("메타데이터 저장 실패: {}", e))?;

    Ok(())
}

/// 격리 기록을 완전히 삭제한다 (폴더 전체 제거).
#[tauri::command]
pub fn quarantine_delete(quarantine_id: String) -> Result<(), String> {
    let dir = quarantine_dir()?.join(&quarantine_id);
    if !dir.exists() {
        return Ok(()); // 이미 없으면 성공으로 처리
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("격리 기록 삭제 실패: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_record(base: &Path, id: &str, restored: bool) {
        let dir = base.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        let meta = QuarantineMetadata {
            id: id.to_string(),
            check_id: "uac".into(),
            action_label: "UAC 복원".into(),
            created_at: "2026-06-30T12:00:00Z".into(),
            restored,
        };
        std::fs::write(
            dir.join("metadata.json"),
            serde_json::to_string(&meta).unwrap(),
        )
        .unwrap();
        let entries: Vec<RegBackupEntry> = vec![RegBackupEntry {
            hive: "HKLM".into(),
            path: "SOFTWARE\\Test".into(),
            name: "EnableLUA".into(),
            value_type: "DWORD".into(),
            original_value: serde_json::json!(0),
        }];
        std::fs::write(
            dir.join("registry.json"),
            serde_json::to_string(&entries).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn quarantine_save_creates_files() {
        let id = quarantine_save(
            vec![RegBackupEntry {
                hive: "HKLM".into(),
                path: "SOFTWARE\\Test".into(),
                name: "EnableLUA".into(),
                value_type: "DWORD".into(),
                original_value: serde_json::json!(1),
            }],
            "uac".into(),
            "UAC 복원".into(),
        )
        .expect("quarantine_save 실패");

        let dir = quarantine_dir().unwrap().join(&id);
        assert!(dir.join("metadata.json").exists());
        assert!(dir.join("registry.json").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quarantine_dir_resolves_under_localappdata() {
        let dir = quarantine_dir().expect("quarantine_dir 실패");
        assert!(dir.ends_with("WinGuard\\quarantine") || dir.ends_with("WinGuard/quarantine"));
    }

    #[test]
    fn read_record_parses_valid_directory() {
        let tmp = TempDir::new().unwrap();
        write_record(&tmp.path().to_path_buf(), "1000", false);
        let record = read_record(&tmp.path().join("1000"));
        assert!(record.is_some());
        assert_eq!(record.unwrap().metadata.check_id, "uac");
    }

    #[test]
    fn read_record_returns_none_for_missing_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("empty");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read_record(&dir).is_none());
    }

    #[test]
    fn prune_removes_restored_entries_first_when_over_threshold() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_path_buf();
        let mut records = Vec::new();
        for i in 0..(PRUNE_THRESHOLD + 5) {
            let id = format!("{:013}", i);
            let restored = i < 3; // 처음 3개만 restored=true
            write_record(&base, &id, restored);
            records.push(read_record(&base.join(&id)).unwrap());
        }
        let before = records.len();
        prune_if_needed(&base, &mut records);
        assert_eq!(records.len(), before - PRUNE_BATCH);
        // restored=true 였던 3개는 모두 삭제 대상에 포함되어야 함
        let remaining_restored = records.iter().filter(|r| r.metadata.restored).count();
        assert_eq!(remaining_restored, 0);
    }

    #[test]
    fn never_panics_regardless_of_system_state() {
        let _ = quarantine_list();
        let _ = quarantine_restore("nonexistent-id-xyz".into());
        let _ = quarantine_delete("nonexistent-id-xyz".into());
        let _ = read_reg_dword("HKLM".into(), "SOFTWARE\\Test".into(), "X".into());
        let _ = read_reg_dword("UNKNOWN".into(), "SOFTWARE\\Test".into(), "X".into());
    }

    #[test]
    fn read_reg_dword_rejects_unknown_hive() {
        let r = read_reg_dword("HKCR".into(), "SOFTWARE\\Test".into(), "X".into());
        assert!(r.is_err());
    }
}
