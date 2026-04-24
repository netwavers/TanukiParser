@echo off
rem TanukiParser Linux Build Wrapper
rem Created by Antigravity

echo TanukiParser Linux Build (WSL) starting...
wsl bash ./build_linux.sh

if %ERRORLEVEL% EQU 0 (
    echo Build completed successfully.
    echo Binary: target\x86_64-unknown-linux-musl\release\TanukiParser
) else (
    echo Build failed.
)
