use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// CREATE_NO_WINDOW: PowerShell 창이 화면에 표시되지 않도록 한다.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 한국어 Windows 에서 PowerShell 표준 출력이 CP949 로 나올 수 있으므로
/// 모든 스크립트 앞에 UTF-8 인코딩 설정을 자동 삽입한다.
const UTF8_PREAMBLE: &str = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
     $OutputEncoding = [System.Text.Encoding]::UTF8; ";

pub fn run(script: &str) -> Result<String, String> {
    let full_script = format!("{}{}", UTF8_PREAMBLE, script);
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &full_script,
    ]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd
        .output()
        .map_err(|e| format!("PowerShell 실행 실패: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "PowerShell 실패 (exit={:?}): {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_panics_on_invalid_script() {
        let _ = run("this_is_not_a_valid_cmdlet_xyzzy");
    }
}
