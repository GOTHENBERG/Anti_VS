@echo off
chcp 65001 >nul
echo ======================================================
echo   Сборка Anti_VS и упаковка в ZIP для отправки другу
echo ======================================================
echo.

echo 1. Компиляция в Release-режиме (максимальная производительность)...
cargo build --release
if errorlevel 1 (
    echo.
    echo [ОШИБКА] Сборка завершилась с ошибкой!
    pause
    exit /b %errorlevel%
)

echo.
echo 2. Подготовка файлов релиза в папке dist\Anti_VS_Game...
if exist dist rmdir /s /q dist
mkdir dist\Anti_VS_Game

copy /y target\release\Anti_VS.exe dist\Anti_VS_Game\Anti_VS.exe >nul
xcopy /e /i /y assets dist\Anti_VS_Game\assets >nul
xcopy /e /i /y models dist\Anti_VS_Game\models >nul

echo.
echo 3. Сжатие в готовый архив dist\Anti_VS_Release.zip...
powershell -Command "Compress-Archive -Path 'dist\Anti_VS_Game\*' -DestinationPath 'dist\Anti_VS_Release.zip' -Force"

echo.
echo ======================================================
echo   Успешно! Архив для отправки другу готов:
echo   C:\Anti_VS\dist\Anti_VS_Release.zip
echo.
echo   Другу нужно лишь:
echo   1. Распаковать ZIP-архив в любую папку.
echo   2. Запустить Anti_VS.exe
echo ======================================================
pause
