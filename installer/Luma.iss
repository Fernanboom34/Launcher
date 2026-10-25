; Prototipo: no publicar hasta completar y verificar desinstalacion de datos.
#ifndef SetupVersion
  #define SetupVersion "0.7.1"
#endif

[Setup]
AppId={{41EDDB92-F249-4516-9214-32958CA71088}
AppName=Luma Launcher
AppVersion={#SetupVersion}
AppPublisher=Luma
AppPublisherURL=https://github.com/Fernanboom34/Launcher
DefaultDirName={autopf}\Luma Launcher
DefaultGroupName=Luma Launcher
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\artifacts\installer-draft
OutputBaseFilename=Luma-Setup-DRAFT
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\Luma.exe
SetupLogging=yes

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear acceso directo en el escritorio"; Flags: unchecked

[Files]
Source: "{tmp}\Luma.exe"; DestDir: "{app}"; Flags: external ignoreversion

[Icons]
Name: "{group}\Luma Launcher"; Filename: "{app}\Luma.exe"
Name: "{autodesktop}\Luma Launcher"; Filename: "{app}\Luma.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Luma.exe"; Description: "Abrir Luma"; Flags: nowait postinstall skipifsilent runasoriginaluser

[Code]
var
  DownloadPage: TDownloadWizardPage;

function JsonString(const Data, Key: String): String;
var
  At, Finish: Integer;
  Tail: String;
begin
  At := Pos('"' + Key + '"', Data);
  if At = 0 then RaiseException('El manifiesto no contiene ' + Key);
  Tail := Trim(Copy(Data, At + Length(Key) + 2, Length(Data)));
  if (Length(Tail) = 0) or (Tail[1] <> ':') then RaiseException('Manifiesto no valido');
  Tail := Trim(Copy(Tail, 2, Length(Tail)));
  if (Length(Tail) = 0) or (Tail[1] <> '"') then RaiseException('Manifiesto no valido');
  Tail := Copy(Tail, 2, Length(Tail));
  Finish := Pos('"', Tail);
  if Finish = 0 then RaiseException('Manifiesto incompleto');
  Result := Copy(Tail, 1, Finish - 1);
  if Pos('\', Result) <> 0 then RaiseException('Valor de manifiesto no admitido');
end;

function VersionNumber(Value: String): Int64;
var
  Part, At, J, N: Integer;
  Digits: String;
begin
  Result := 0;
  for Part := 1 to 3 do begin
    At := Pos('.', Value);
    if Part < 3 then begin
      if At = 0 then RaiseException('Version no valida');
      Digits := Copy(Value, 1, At - 1);
      Value := Copy(Value, At + 1, Length(Value));
    end else begin
      if At <> 0 then RaiseException('Version no valida');
      Digits := Value;
    end;
    if (Length(Digits) = 0) or (Length(Digits) > 3) then RaiseException('Version no valida');
    for J := 1 to Length(Digits) do
      if (Digits[J] < '0') or (Digits[J] > '9') then RaiseException('Version no valida');
    N := StrToIntDef(Digits, -1);
    if N < 0 then RaiseException('Version no valida');
    Result := Result * 1000 + N;
  end;
end;

function ManifestProgress(const Url, FileName: String; const Progress, ProgressMax: Int64): Boolean;
begin
  Result := (Progress <= 8192) and (ProgressMax <= 8192);
end;

procedure InitializeWizard;
begin
  DownloadPage := CreateDownloadPage('Descargando Luma', 'Se verificara la descarga antes de instalar.', nil);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Raw: AnsiString;
  Manifest, Version, Url, Hash, ExpectedUrl: String;
  I: Integer;
begin
  Result := '';
  DownloadPage.Show;
  try
    try
      DownloadTemporaryFile('https://github.com/Fernanboom34/Launcher/releases/latest/download/luma-release.json',
        'luma-release.json', '', @ManifestProgress);
      if not LoadStringFromFile(ExpandConstant('{tmp}\luma-release.json'), Raw) then
        RaiseException('No se pudo leer el manifiesto');
      Manifest := Raw;
      Version := JsonString(Manifest, 'version');
      if VersionNumber(Version) < VersionNumber('{#SetupVersion}') then
        RaiseException('Todavia no hay un release compatible con este instalador de prueba.');
      Url := JsonString(Manifest, 'url');
      Hash := JsonString(Manifest, 'sha256');
      ExpectedUrl := 'https://github.com/Fernanboom34/Launcher/releases/download/v' + Version + '/Luma-' + Version + '.exe';
      if Url <> ExpectedUrl then RaiseException('La descarga no pertenece al release esperado de Luma');
      if Length(Hash) <> 64 then RaiseException('SHA-256 no valido');
      for I := 1 to Length(Hash) do
        if Pos(Lowercase(Hash[I]), '0123456789abcdef') = 0 then RaiseException('SHA-256 no valido');
      DownloadPage.Clear;
      DownloadPage.Add(Url, 'Luma.exe', Hash);
      DownloadPage.Download;
    except
      Result := GetExceptionMessage;
    end;
  finally
    DownloadPage.Hide;
  end;
end;
