; Inno Setup script for the PawPal installer, built by the release workflow:
;   iscc /DVersion=1.0.0 /DSource=<folder with pawpal.exe and pawpal.ico> packaging\windows\pawpal.iss
; Installs for the current user only, so no administrator prompt.

[Setup]
AppId={{CBD68177-13B2-4165-89D2-E5850A8E9EAF}
AppName=PawPal
AppVersion={#Version}
AppPublisher=PawPal
DefaultDirName={localappdata}\Programs\PawPal
DefaultGroupName=PawPal
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputBaseFilename=PawPal-{#Version}-Windows-Setup
SetupIconFile={#Source}\pawpal.ico
UninstallDisplayIcon={app}\pawpal.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "{#Source}\pawpal.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\pawpal.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\PawPal"; Filename: "{app}\pawpal.exe"; IconFilename: "{app}\pawpal.ico"
Name: "{group}\Uninstall PawPal"; Filename: "{uninstallexe}"
Name: "{autodesktop}\PawPal"; Filename: "{app}\pawpal.exe"; IconFilename: "{app}\pawpal.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\pawpal.exe"; Description: "Launch PawPal"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{cmd}"; Parameters: "/C taskkill /IM pawpal.exe /F"; Flags: runhidden; RunOnceId: "StopPawPal"
