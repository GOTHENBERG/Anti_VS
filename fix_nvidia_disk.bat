@echo off
chcp 65001 >nul
echo ========================================================
echo Исправление бага драйвера NVIDIA (запись в nvlog.nvlgstg)
echo ========================================================
echo.

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [ВНИМАНИЕ] Нужны права администратора!
    echo Нажмите правой кнопкой мыши по этому файлу и выберите "Запуск от имени администратора".
    echo.
    pause
    exit /b 1
)

echo Отключаем проблемный модуль nvtopps в реестре...
reg add "HKLM\SYSTEM\CurrentControlSet\Services\NVDisplay.ContainerLocalSystem\Parameters\LocalSystem\_nvtopps" /v DisableLoad /t REG_DWORD /d 1 /f >nul 2>&1
reg add "HKLM\SYSTEM\CurrentControlSet\Services\NVDisplay.ContainerLocalSystem\Parameters\LocalSystem\NvTopps" /v DisableLoad /t REG_DWORD /d 1 /f >nul 2>&1

echo Перезапускаем службу NVIDIA Display Container...
net stop NVDisplay.ContainerLocalSystem >nul 2>&1
net start NVDisplay.ContainerLocalSystem >nul 2>&1

echo Очищаем мусорный файл логов...
del /f /q "C:\ProgramData\NVIDIA Corporation\nvtopps\nct\nvlog.nvlgstg" >nul 2>&1

echo.
echo ========================================================
echo [ГОТОВО] Баг устранен! 
echo Служба NVIDIA больше не насилует SSD постоянной записью.
echo ========================================================
echo.
pause
