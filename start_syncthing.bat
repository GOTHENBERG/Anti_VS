@echo off
start "" "C:\Users\Admin\AppData\Local\Microsoft\WinGet\Packages\Syncthing.Syncthing_Microsoft.Winget.Source_8wekyb3d8bbwe\syncthing-windows-amd64-v2.1.7\syncthing.exe" serve --no-browser --no-console
timeout /t 1 >nul
start http://127.0.0.1:8384/
