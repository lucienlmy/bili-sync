@echo off
rem bili-sync 交互启动脚本（可双击运行）

@echo off
rem bili-sync 交互启动脚本（可双击运行）

rem 尝试使用 UTF-8 以改善中文显示（可按需移除）
chcp 65001 >nul

setlocal enabledelayedexpansion

rem 保存并清空提示符，避免显示路径前缀
set "OLD_PROMPT=%PROMPT%"
set "PROMPT="

set "ROOT=%~dp0"
rem Prefer an executable in the same directory as this script; otherwise use the hardcoded target path
set "LOCAL_EXE=%ROOT%bili-sync-rs.exe"
set "HARDCODED_EXE=%ROOT%target\release\bili-sync-rs.exe"
if exist "%LOCAL_EXE%" (
  set "EXE=%LOCAL_EXE%"
) else (
  set "EXE=%HARDCODED_EXE%"
)

if not exist "%EXE%" (
  echo 找不到可执行文件："%EXE%"
  pause
  if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
  endlocal
  exit /b 1
)

:menu
cls
"%EXE%" -V
echo.
echo ====================
echo Bili-Sync 启动器
echo.
echo 0) 无参数运行（直接运行可执行文件）
echo 1) 只运行一次（--run-once / -r）

echo 2) 仅扫描（--scan-only / -s)
echo 3) 日志级别 (-l, 需要输入值)
echo 4) 禁用凭证刷新 (--disable-credential-refresh / -d)
echo 5) 设置配置目录 (-c, 需要输入值)
echo 6) 设置 ffmpeg 路径 (-f, 需要输入值)
echo 7) 帮助 (-h)
echo 8) 版本 (-V)
echo 9) 退出
echo.

rem 使用 PowerShell 读取单键（支持 Tab），无需回车
echo 按键选择（数字键、0 或 Tab 键；按 Tab 可直接运行）：
for /f "usebackq delims=" %%K in (`powershell -nologo -noprofile -command "$k=[System.Console]::ReadKey($true); if($k.Key -eq 'Tab'){ Write-Host 'TAB' } else { Write-Host $k.KeyChar }"`) do set "KEY=%%K"
if /I "%KEY%"=="TAB" set "CHOICE=0"
if "%KEY%"=="0" set "CHOICE=0"
if "%KEY%"=="1" set "CHOICE=1"
if "%KEY%"=="2" set "CHOICE=2"
if "%KEY%"=="3" set "CHOICE=3"
if "%KEY%"=="4" set "CHOICE=4"
if "%KEY%"=="5" set "CHOICE=5"
if "%KEY%"=="6" set "CHOICE=6"
if "%KEY%"=="7" set "CHOICE=7"
if "%KEY%"=="8" set "CHOICE=8"
if "%KEY%"=="9" set "CHOICE=9"

if "%CHOICE%"=="9" (
  if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
  endlocal
  exit /b 0
)
if "%CHOICE%"=="0" goto run_noargs
if "%CHOICE%"=="1" goto run_r
if "%CHOICE%"=="2" goto run_s
if "%CHOICE%"=="3" goto run_l
if "%CHOICE%"=="4" goto run_d
if "%CHOICE%"=="5" goto run_c
if "%CHOICE%"=="6" goto run_f
if "%CHOICE%"=="7" goto show_help
if "%CHOICE%"=="8" goto show_version

echo 无效选择。
goto menu

:run_noargs
echo 正在运行："%EXE%"
"%EXE%"
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_r
echo 正在运行："%EXE%" -r
"%EXE%" -r
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_s
echo 正在运行："%EXE%" --scan-only
"%EXE%" --scan-only
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_l
set /p "LL=输入日志级别（例如 None,bili_sync=info）："
if "%LL%"=="" (
  echo 未输入日志级别，返回菜单。
  goto menu
)
echo 正在运行："%EXE%" -l "%LL%"
"%EXE%" -l "%LL%"
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_d
echo 正在运行："%EXE%" --disable-credential-refresh
"%EXE%" --disable-credential-refresh
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_c
set /p "CFG=输入配置目录（完整路径）："
if "%CFG%"=="" (
  echo 未输入配置目录，返回菜单。
  goto menu
)
echo 正在运行："%EXE%" -c "%CFG%"
"%EXE%" -c "%CFG%"
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_f
set /p "FFP=输入 ffmpeg.exe 完整路径："
if "%FFP%"=="" (
  echo 未输入 ffmpeg 路径，返回菜单。
  goto menu
)
echo 正在运行："%EXE%" -f "%FFP%"
"%EXE%" -f "%FFP%"
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:show_help
"%EXE%" -h
echo.
pause
goto menu

:show_version
"%EXE%" -V
echo.
pause
goto menu
exit /b %RC%

:run_d
echo 正在运行："%EXE%" --disable-credential-refresh
"%EXE%" --disable-credential-refresh
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_c
set /p "CFG=输入配置目录（完整路径）："
if "%CFG%"=="" (
  echo 未输入配置目录，返回菜单。
  goto menu
)
echo 正在运行："%EXE%" -c "%CFG%"
"%EXE%" -c "%CFG%"
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:run_f
set /p "FFP=输入 ffmpeg.exe 完整路径："
if "%FFP%"=="" (
  echo 未输入 ffmpeg 路径，返回菜单。
  goto menu
)
echo 正在运行："%EXE%" -f "%FFP%"
"%EXE%" -f "%FFP%"
set "RC=%ERRORLEVEL%"
echo 进程已退出，错误码 %RC%
pause
if defined OLD_PROMPT set "PROMPT=%OLD_PROMPT%"
endlocal
exit /b %RC%

:show_help
"%EXE%" -h
echo.
pause
goto menu

:show_version
"%EXE%" -V
echo.
pause
goto menu