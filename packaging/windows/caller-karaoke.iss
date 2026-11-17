#define AppName "Caller Karaoke"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef OutputBaseFilename
  #define OutputBaseFilename "CallerKaraoke-0.1.0-x64-setup"
#endif

[Setup]
AppId={{6E1E7F5A-9C2B-4D60-9F2A-0D6C2A6D7C11}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Nicholas Gower
AppPublisherURL=https://github.com/nicholasgower/Caller-karaoke
AppSupportURL=https://github.com/nicholasgower/Caller-karaoke/issues
DefaultDirName={autopf}\Caller Karaoke
DefaultGroupName=Caller Karaoke
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename={#OutputBaseFilename}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\caller-karaoke.exe
SetupIconFile=..\..\assets\icons\caller-karaoke.ico

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Files]
Source: "..\..\target\release\caller-karaoke.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\icons\caller-karaoke.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\CREDITS.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Caller Karaoke"; Filename: "{app}\caller-karaoke.exe"; IconFilename: "{app}\caller-karaoke.ico"
Name: "{group}\Credits"; Filename: "{app}\CREDITS.md"
Name: "{autodesktop}\Caller Karaoke"; Filename: "{app}\caller-karaoke.exe"; IconFilename: "{app}\caller-karaoke.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\caller-karaoke.exe"; Description: "Launch Caller Karaoke"; Flags: nowait postinstall skipifsilent
