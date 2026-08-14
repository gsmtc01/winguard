@echo off
setlocal
cd /d "%~dp0"

REM WinGuard 릴리스 빌드 — x64 / ARM64 포터블 실행 파일 생성
REM 결과물: release\WinGuard-x64.exe , release\WinGuard-arm64.exe

if not defined VCVARS set "VCVARS=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat"
if not exist "%VCVARS%" (
  echo [오류] vcvarsall.bat 을 찾을 수 없습니다:
  echo        "%VCVARS%"
  echo        Visual Studio 설치 경로가 다르면 VCVARS 환경변수로 지정하세요.
  exit /b 1
)

set "PATH=C:\Program Files\LLVM\bin;%PATH%"
taskkill /F /IM winguard.exe >nul 2>&1

echo.
echo ===== [1/2] x64 빌드 =====
cmd /c ""%VCVARS%" amd64 && pnpm tauri build --target x86_64-pc-windows-msvc"
if errorlevel 1 (
  echo [실패] x64 빌드
  exit /b 1
)

echo.
echo ===== [2/2] ARM64 빌드 =====
cmd /c ""%VCVARS%" amd64_arm64 && pnpm tauri build --target aarch64-pc-windows-msvc"
if errorlevel 1 (
  echo [실패] ARM64 빌드
  exit /b 1
)

if not exist "release" mkdir "release"
copy /Y "src-tauri\target\x86_64-pc-windows-msvc\release\winguard.exe"  "release\WinGuard-x64.exe"   >nul || exit /b 1
copy /Y "src-tauri\target\aarch64-pc-windows-msvc\release\winguard.exe" "release\WinGuard-arm64.exe" >nul || exit /b 1

echo.
echo ===== 빌드 완료 =====
dir /b release
endlocal
