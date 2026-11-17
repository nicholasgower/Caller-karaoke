#define AppName "Contra Karaoke"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef OutputBaseFilename
  #define OutputBaseFilename "ContraKaraoke-0.1.0-x64-setup"
#endif

[Setup]
AppId={{6E1E7F5A-9C2B-4D60-9F2A-0D6C2A6D7C11}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Nicholas Gower
AppPublisherURL=https://github.com/nicholasgower/Contra-karaoke
AppSupportURL=https://github.com/nicholasgower/Contra-karaoke/issues
DefaultDirName={autopf}\Contra Karaoke
DefaultGroupName=Contra Karaoke
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename={#OutputBaseFilename}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\contra-karaoke.exe

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Files]
Source: "..\..\target\release\contra-karaoke.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\CREDITS.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Contra Karaoke"; Filename: "{app}\contra-karaoke.exe"
Name: "{group}\Credits"; Filename: "{app}\CREDITS.md"
Name: "{autodesktop}\Contra Karaoke"; Filename: "{app}\contra-karaoke.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\contra-karaoke.exe"; Description: "Launch Contra Karaoke"; Flags: nowait postinstall skipifsilent
