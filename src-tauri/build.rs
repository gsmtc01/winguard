fn main() {
    // Release 빌드에서만 관리자 권한 매니페스트를 포함한다.
    // cargo test(debug 프로파일)는 기본 매니페스트를 사용해 SxS 오류를 방지한다.
    let profile = std::env::var("PROFILE").unwrap_or_default();

    if profile == "release" {
        let windows_attrs =
            tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest"));
        let attrs = tauri_build::Attributes::new().windows_attributes(windows_attrs);
        tauri_build::try_build(attrs).expect("failed to run tauri-build");
    } else {
        tauri_build::build();
    }
}
