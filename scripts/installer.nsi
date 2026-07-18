; Overseer Windows Installer Script (NSIS)
; Requires NSIS Modern UI 2 (MUI2)

!define PRODUCT_NAME "Overseer"
!define PRODUCT_PUBLISHER "Revaycolizer"
!define PRODUCT_EXE "Overseer.exe"

SetCompressor /SOLID lzma
RequestExecutionLevel user
ManifestDPIAware true
SetFont "Segoe UI" 9
BrandingText "Overseer Mission Control"

; --------------------------------
; Modern UI Configuration
; --------------------------------
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "${REPO_ROOT}\assets\installer_header.bmp"
!define MUI_HEADERIMAGE_UNBITMAP "${REPO_ROOT}\assets\installer_header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "${REPO_ROOT}\assets\installer_welcome.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "${REPO_ROOT}\assets\installer_welcome.bmp"

!include "MUI2.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON "${REPO_ROOT}\assets\favicon.ico"
!define MUI_UNICON "${REPO_ROOT}\assets\favicon.ico"

; Welcome, License, Directory, Install, and Finish Pages
!define MUI_WELCOMEPAGE_TITLE "Welcome to Overseer"
!define MUI_WELCOMEPAGE_TEXT "This assistant will install Overseer, your local-first AI orchestration mission control.\r\n\r\nClick Next to continue."
!insertmacro MUI_PAGE_WELCOME

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!define MUI_FINISHPAGE_RUN "$INSTDIR\${PRODUCT_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Overseer"
!define MUI_FINISHPAGE_TITLE "Installation Complete"
!define MUI_FINISHPAGE_TEXT "Overseer has been successfully installed on your computer.\r\n\r\nLaunch the application to begin orchestrating your local AI agents."
!insertmacro MUI_PAGE_FINISH

; Uninstaller Pages
!define MUI_WELCOMEPAGE_TITLE "Uninstall Overseer"
!define MUI_WELCOMEPAGE_TEXT "This assistant will guide you through the uninstallation of Overseer.\r\n\r\nBefore starting, make sure Overseer is not running."
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

; --------------------------------
; Installer Core Attributes
; --------------------------------
Name "${PRODUCT_NAME}"
OutFile "${OUT_DIR}\Overseer-${TAG}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\${PRODUCT_NAME}"
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "InstallLocation"
ShowInstDetails show
ShowUnInstDetails show

; --------------------------------
; Installation Section
; --------------------------------
Section "Install"
  SetOutPath "$INSTDIR"
  
  ; Write core files
  File "${PORTABLE_DIR}\Overseer.exe"
  File "${PORTABLE_DIR}\CHANGELOG.md"
  File "${PORTABLE_DIR}\README.md"
  
  ; Recursively copy folders
  File /r "${PORTABLE_DIR}\assets"
  File /r "${PORTABLE_DIR}\config"
  File /r "${PORTABLE_DIR}\tool_scripts"
  File /r "${PORTABLE_DIR}\src"
  
  ; Write uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"
  
  ; Create Start Menu Shortcuts
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}"
  CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk" "$INSTDIR\uninstall.exe"
  
  ; Create Desktop Shortcut
  CreateShortcut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}"
  
  ; Write Registry keys for Control Panel / Add-Remove Programs
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayIcon" '"$INSTDIR\${PRODUCT_EXE}"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "NoRepair" 1
SectionEnd

; --------------------------------
; Uninstaller Section
; --------------------------------
Section "Uninstall"
  ; Delete executable and assets
  Delete "$INSTDIR\${PRODUCT_EXE}"
  Delete "$INSTDIR\CHANGELOG.md"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\uninstall.exe"
  
  RMDir /r "$INSTDIR\assets"
  RMDir /r "$INSTDIR\config"
  RMDir /r "$INSTDIR\tool_scripts"
  RMDir /r "$INSTDIR\src"
  RMDir "$INSTDIR"
  
  ; Remove Shortcuts
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"
  
  ; Remove registry keys
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
  
  ; Optional clean up of user database and settings
  MessageBox MB_YESNO|MB_ICONQUESTION "Do you want to completely remove your personal database, work items, and configuration files stored in AppData?" IDNO skip_data
    RMDir /r "$APPDATA\overseer"
  skip_data:
SectionEnd
