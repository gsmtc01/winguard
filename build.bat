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
REM 실행 중이면 복사 단계에서 파일이 잠긴다. 먼저 종료를 시도한다.
REM 단, 앱은 requireAdministrator 로 실행되므로 이 스크립트가 관리자 권한이
REM 아니면 taskkill 이 "Access is denied" 로 실패한다. 그래서 아래에서 한 번 더
REM 확인하고, 남아 있으면 빌드를 시작하기 전에 멈춘다(빌드에 몇 분이 걸리는데
REM 끝난 뒤 복사에서 실패하면 그 시간을 통째로 버리게 된다).
taskkill /F /IM winguard.exe >nul 2>&1
taskkill /F /IM WinGuard-x64.exe >nul 2>&1
taskkill /F /IM WinGuard-arm64.exe >nul 2>&1

for %%P in (winguard.exe WinGuard-x64.exe WinGuard-arm64.exe) do (
  tasklist /FI "IMAGENAME eq %%P" 2>nul | find /I "%%P" >nul && (
    echo.
    echo [오류] %%P 이^(가^) 아직 실행 중입니다.
    echo        관리자 권한으로 실행된 앱은 이 스크립트에서 종료할 수 없습니다.
    echo        앱을 직접 닫은 뒤 다시 실행하세요.
    exit /b 1
  )
)

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
REM 호스트가 ARM64 이므로 네이티브 빌드. Hostarm64\arm64\cl.exe (ARM64 바이너리)를
REM 쓰려면 amd64_arm64 가 아니라 arm64 를 지정해야 한다.
REM amd64_arm64 는 Hostx64\arm64\cl.exe (x64 바이너리)라 에뮬레이션으로 돈다.
cmd /c ""!VCVARS!" arm64 && pnpm tauri build --target aarch64-pc-windows-msvc"
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
