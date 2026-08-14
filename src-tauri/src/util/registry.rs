#[cfg(windows)]
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
#[cfg(windows)]
use winreg::RegKey;

#[cfg(windows)]
pub fn read_dword(path: &str, name: &str) -> Result<u32, String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(path)
        .map_err(|e| format!("레지스트리 키 열기 실패: path={}, error={}", path, e))?;
    key.get_value::<u32, _>(name)
        .map_err(|e| format!("DWORD 읽기 실패: path={}, name={}, error={}", path, name, e))
}

#[cfg(windows)]
pub fn read_string(path: &str, name: &str) -> Result<String, String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(path)
        .map_err(|e| format!("레지스트리 키 열기 실패: path={}, error={}", path, e))?;
    key.get_value::<String, _>(name).map_err(|e| {
        format!(
            "문자열 읽기 실패: path={}, name={}, error={}",
            path, name, e
        )
    })
}

#[cfg(windows)]
pub fn read_dword_hkcu(path: &str, name: &str) -> Result<u32, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(path)
        .map_err(|e| format!("레지스트리 키 열기 실패: path={}, error={}", path, e))?;
    key.get_value::<u32, _>(name)
        .map_err(|e| format!("DWORD 읽기 실패: path={}, name={}, error={}", path, name, e))
}

#[cfg(windows)]
pub fn read_string_hkcu(path: &str, name: &str) -> Result<String, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(path)
        .map_err(|e| format!("레지스트리 키 열기 실패: path={}, error={}", path, e))?;
    key.get_value::<String, _>(name).map_err(|e| {
        format!(
            "문자열 읽기 실패: path={}, name={}, error={}",
            path, name, e
        )
    })
}

#[cfg(not(windows))]
pub fn read_dword(_path: &str, _name: &str) -> Result<u32, String> {
    Err("레지스트리는 Windows 에서만 지원됩니다.".into())
}

#[cfg(not(windows))]
pub fn read_dword_hkcu(_path: &str, _name: &str) -> Result<u32, String> {
    Err("레지스트리는 Windows 에서만 지원됩니다.".into())
}

#[cfg(not(windows))]
pub fn read_string(_path: &str, _name: &str) -> Result<String, String> {
    Err("레지스트리는 Windows 에서만 지원됩니다.".into())
}

#[cfg(not(windows))]
pub fn read_string_hkcu(_path: &str, _name: &str) -> Result<String, String> {
    Err("레지스트리는 Windows 에서만 지원됩니다.".into())
}
