Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel user

!include MUI2.nsh
!include FileFunc.nsh
!include LogicLib.nsh

!ifndef VERSION
  !error "Pass /DVERSION=a.b.c.d"
!endif
!ifndef STAGE
  !error "Pass /DSTAGE=<folder with nuru.exe, nuru-restart.exe, icon.ico and packs>"
!endif
!ifndef OUTFILE
  !define OUTFILE "Nuru_${VERSION}_x64-setup.exe"
!endif

!define PRODUCT "Nuru"
!define PUBLISHER "SimpleFox"
!define EXE "nuru.exe"
!define APP_KEY "Software\SimpleFox\Nuru"
!define UNINSTALL_ROOT "Software\Microsoft\Windows\CurrentVersion\Uninstall"
!define TEST_UNINSTALL_ROOT "Software\Hexagon Cloud Test\Uninstall"

Name "${PRODUCT}"
OutFile "${OUTFILE}"
BrandingText "${PRODUCT} ${VERSION}"
VIProductVersion "${VERSION}"
VIAddVersionKey "ProductName" "${PRODUCT}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${PRODUCT} setup"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 ${PUBLISHER}"

!define MUI_ICON "${STAGE}\icon.ico"
!define MUI_UNICON "${STAGE}\icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"

Var UninstallRoot
Var TestMode
Var Relaunch

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

!macro PICK_ROOTS
  ReadEnvStr $0 HEXAGON_CLOUD_PROGRAMS
  ${If} $0 != ""
    StrCpy $TestMode 1
    StrCpy $UninstallRoot "${TEST_UNINSTALL_ROOT}"
  ${Else}
    StrCpy $TestMode 0
    StrCpy $UninstallRoot "${UNINSTALL_ROOT}"
  ${EndIf}
!macroend

!macro CLOSE_RUNNING
  StrCpy $5 0
  ${If} ${FileExists} "$INSTDIR\${EXE}"
    ${Do}
      ClearErrors
      FileOpen $1 "$INSTDIR\${EXE}" a
      ${IfNot} ${Errors}
        FileClose $1
        ${Break}
      ${EndIf}
      ${If} ${Silent}
        ${If} $5 == 0
          nsExec::Exec `"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -Command "Get-Process | Where-Object { $$_.Path -eq '$INSTDIR\${EXE}' } | Stop-Process -Force"`
        ${EndIf}
        IntOp $5 $5 + 1
        ${If} $5 > 20
          ${Break}
        ${EndIf}
        Sleep 500
      ${Else}
        MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Close ${PRODUCT} to continue." IDRETRY +2
        Abort
      ${EndIf}
    ${Loop}
  ${EndIf}
!macroend

!macro DELETE_RETRY FILE
  StrCpy $5 0
  ${Do}
    Delete "${FILE}"
    ${IfNot} ${FileExists} "${FILE}"
      ${Break}
    ${EndIf}
    IntOp $5 $5 + 1
    ${If} $5 > 20
      ${Break}
    ${EndIf}
    Sleep 500
  ${Loop}
!macroend

Function .onInit
  !insertmacro PICK_ROOTS
  ReadEnvStr $0 LOCALAPPDATA
  StrCpy $INSTDIR "$0\Programs\${PRODUCT}"
  ${If} $TestMode == 0
    ReadRegStr $1 HKCU "${APP_KEY}" ""
    ${If} $1 != ""
      StrCpy $INSTDIR $1
    ${EndIf}
  ${EndIf}

  StrCpy $Relaunch 0
  ${GetParameters} $2
  ClearErrors
  ${GetOptions} $2 "/R" $3
  ${IfNot} ${Errors}
    StrCpy $Relaunch 1
  ${EndIf}
  ReadEnvStr $3 HEXAGON_CLOUD_INSTALL
  ReadEnvStr $4 APPDATA
  ${If} $3 == "1"
  ${AndIf} ${FileExists} "$4\app.nuru.player\updating.json"
    StrCpy $Relaunch 1
  ${EndIf}
FunctionEnd

Section "Install"
  SetOutPath "$INSTDIR"
  !insertmacro CLOSE_RUNNING

  File "${STAGE}\${EXE}"
  File "${STAGE}\nuru-restart.exe"
  RMDir /r "$INSTDIR\packs"
  SetOutPath "$INSTDIR\packs"
  File /r "${STAGE}\packs\*.*"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ${If} $TestMode == 0
    WriteRegStr HKCU "${APP_KEY}" "" "$INSTDIR"
    CreateShortcut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\${EXE}"
  ${EndIf}

  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "DisplayName" "${PRODUCT}"
  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "DisplayIcon" '"$INSTDIR\${EXE}"'
  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "InstallLocation" '"$INSTDIR"'
  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "$UninstallRoot\${PRODUCT}" "MainBinaryName" "${EXE}"
  WriteRegDWORD HKCU "$UninstallRoot\${PRODUCT}" "NoModify" 1
  WriteRegDWORD HKCU "$UninstallRoot\${PRODUCT}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "$UninstallRoot\${PRODUCT}" "EstimatedSize" "$0"

  ${If} $Relaunch == 1
    Exec '"$INSTDIR\${EXE}"'
  ${EndIf}
SectionEnd

Function un.onInit
  !insertmacro PICK_ROOTS
FunctionEnd

Section "Uninstall"
  !insertmacro CLOSE_RUNNING
  !insertmacro DELETE_RETRY "$INSTDIR\${EXE}"
  !insertmacro DELETE_RETRY "$INSTDIR\nuru-restart.exe"
  RMDir /r "$INSTDIR\packs"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  ${If} $TestMode == 0
    Delete "$SMPROGRAMS\${PRODUCT}.lnk"
    Delete "$DESKTOP\${PRODUCT}.lnk"
    DeleteRegKey HKCU "${APP_KEY}"
  ${EndIf}
  DeleteRegKey HKCU "$UninstallRoot\${PRODUCT}"
SectionEnd
