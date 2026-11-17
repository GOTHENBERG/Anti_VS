@echo off
chcp 65001 >nul
echo ======================================================
echo   Отправка обученных моделей ИИ в GitHub (origin/master)
echo ======================================================
echo.
git status -s
echo.
git add models/
git commit -m "train: update AI models checkpoint"
git push origin master
echo.
echo ======================================================
echo   Готово! Веса успешно запушены в GitHub.
echo ======================================================
pause
