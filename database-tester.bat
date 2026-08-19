@echo off

powershell -Command "Start-Process cmd.exe -Verb RunAs -ArgumentList '/k cd /d ""%~dp0"" && npm run db:seed && npm run db:dev'"

exit