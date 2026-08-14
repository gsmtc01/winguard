@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
cd /d "%~dp0"

REM 이 파일은 반드시 CRLF 줄바꿈으로 저장해야 한다 (.gitattributes 참고).
REM LF 로 저장되면 cmd.exe 배치 파서가 줄을 뭉개서 주석까지 실행해 버린다.
REM
REM WinGuard 릴리스 빌드 — x64 / ARM64 포터블 실행 파일 생성
REM   build.bat          두 아키텍처 모두
REM   build.bat x64      x64 만
REM   build.bat arm64    ARM64 만
REM 결과물: release\WinGuard-x64.exe , release\WinGuard-arm64.exe

set "ARCH=%~1"
if "%ARCH%"=="" set "ARCH=all"

REM Visual Studio 빌드 환경 탐지 (VCVARS 환경변수로 재정의 가능).
REM vswhere 출력은 임시 파일로 받는다 — for /f 서브셸에서 공백 있는 경로를
REM 호출하면 cmd 가 따옴표를 벗겨 실행에 실패한다.
REM -products * 는 IDE 없이 Build Tools 만 설치된 경우를 찾기 위해 필요하다.
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VSTMP=%TEMP%\winguard_vspath.txt"
if not defined VCVARS (
  if exist "!VSWHERE!" (
    "!VSWHERE!" -latest -products * -property installationPath > "!VSTMP!" 2>nul
    set /p VSPATH=<"!VSTMP!"
    if defined VSPATH set "VCVARS=!VSPATH!\VC\Auxiliary\Build\vcvarsall.bat"
  )
)
if not defined VCVARS goto :novs
if not exist "!VCVARS!" goto :novs
echo [정보] vcvarsall = !VCVARS!

set "PATH=C:\Program Files\LLVM\bin;%PATH%"
taskkill /F /IM winguard.exe >nul 2>&1

if /i "%ARCH%"=="arm64" goto :arm64

:x64
echo.
echo ===== x64 빌드 =====
REM 호스트가 ARM64 이므로 x64 는 크로스 컴파일.
REM Hostarm64 툴셋에는 x64 타겟이 없어서 x64 호스트 툴셋을 에뮬레이션으로 사용한다.
cmd /c ""!VCVARS!" amd64 && pnpm tauri build --target x86_64-pc-windows-msvc"
if errorlevel 1 (
  echo [실패] x64 빌드
  exit /b 1
)
if not exist "release" mkdir "release"
copy /Y "src-tauri\target\x86_64-pc-windows-msvc\release\winguard.exe" "release\WinGuard-x64.exe" >nul
if errorlevel 1 exit /b 1
if /i "%ARCH%"=="x64" goto :done

:arm64
echo.
echo ===== ARM64 빌드 =====
cmd /c ""!VCVARS!" amd64_arm64 && pnpm tauri build --target aarch64-pc-windows-msvc"
if errorlevel 1 (
  echo [실패] ARM64 빌드
  exit /b 1
)
if not exist "release" mkdir "release"
copy /Y "src-tauri\target\aarch64-pc-windows-msvc\release\winguard.exe" "release\WinGuard-arm64.exe" >nul
if errorlevel 1 exit /b 1

:done
echo.
echo ===== 빌드 완료 =====
dir /b release
endlocal
exit /b 0

:novs
echo [오류] vcvarsall.bat 을 찾을 수 없습니다.
echo        Visual Studio Build Tools 를 설치하거나, VCVARS 환경변수로 경로를 지정하세요.
exit /b 1
