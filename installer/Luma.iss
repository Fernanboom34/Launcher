#ifndef SetupVersion
  #define SetupVersion "0.7.1-beta.2-windows"
#endif
#ifndef ReleaseTag
  #define ReleaseTag "v0.7.1-beta.2-windows"
#endif

[Setup]
AppId={{41EDDB92-F249-4516-9214-32958CA71088}
AppName=Luma Launcher
AppVersion={#SetupVersion}
VersionInfoVersion=0.7.1.0
AppPublisher=Luma
AppPublisherURL=https://github.com/Fernanboom34/Launcher
DefaultDirName={autopf}\Luma Launcher
DefaultGroupName=Luma Launcher
DisableProgramGroupPage=yes
; Let Inno perform its standard elevation. Launching the same Setup from
; InitializeSetup is explicitly unsupported by Inno's Exec/ShellExec APIs.
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\artifacts
OutputBaseFilename=Luma-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\Luma.exe
SetupLogging=yes
DisableDirPage=yes
UsePreviousAppDir=no

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear acceso directo en el escritorio"; Flags: unchecked

[Files]
#ifdef FixturePayload
Source: "{#FixturePayload}"; DestDir: "{app}"; DestName: "Luma.exe"; Flags: ignoreversion
#else
Source: "{tmp}\Luma.exe"; DestDir: "{app}"; Flags: external ignoreversion
#endif

[InstallDelete]
Type: files; Name: "{app}\Luma-Setup.exe"

[UninstallDelete]
Type: files; Name: "{app}\Luma.exe.previous"
Type: files; Name: "{app}\Luma.download"

[Icons]
Name: "{group}\Luma Launcher"; Filename: "{app}\Luma.exe"
Name: "{autodesktop}\Luma Launcher"; Filename: "{app}\Luma.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Luma.exe"; Description: "Abrir Luma"; Flags: nowait postinstall skipifsilent runasoriginaluser

[Code]
var
  DownloadPage: TDownloadWizardPage;

function DownloadMaintenancePayload: String; forward;

function NativeFileAttributes(const Name: String): LongWord;
  external 'GetFileAttributesW@kernel32.dll stdcall';

procedure RemoveLegacyProgramFiles(const AppDir: String);
var
  Names: TArrayOfString;
  I: Integer;
  FileName: String;
begin
  // Only exact application files in the fixed install directory. Unknown
  // content and player data are never recursively deleted by this code.
  if DirExists(AppDir) and ((NativeFileAttributes(AppDir) and $400) <> 0) then
    RaiseException('La carpeta de instalacion es un enlace. No se elimino.');
  Names := ['Luma.exe', 'Luma-Setup.exe', 'Luma.exe.previous', 'Luma.download'];
  for I := 0 to GetArrayLength(Names) - 1 do begin
    FileName := AddBackslash(AppDir) + Names[I];
    if FileExists(FileName) and not DeleteFile(FileName) then
      RaiseException('No se pudo retirar ' + FileName + '. Cierra Luma y vuelve a intentarlo.');
  end;
  RemoveDir(AppDir);
end;

function ChooseAction: Integer;
var
  Form: TSetupForm;
  Text: TNewStaticText;
  InstallButton, RemoveButton, CancelButton: TNewButton;
begin
  Form := CreateCustomForm(ScaleX(540), ScaleY(200), False, False);
  try
    Form.Caption := 'Luma Launcher · {#SetupVersion}';
    Form.Position := poScreenCenter;
    Text := TNewStaticText.Create(Form);
    Text.Parent := Form;
    Text.SetBounds(ScaleX(24), ScaleY(24), ScaleX(492), ScaleY(92));
    Text.AutoSize := False;
    Text.WordWrap := True;
    Text.Caption := 'Instala o actualiza Luma en Program Files. Al desinstalar podras conservar tus mundos o eliminar los datos confirmados de tu usuario.';
    InstallButton := TNewButton.Create(Form);
    InstallButton.Parent := Form;
    InstallButton.SetBounds(ScaleX(24), ScaleY(132), ScaleX(184), ScaleY(36));
    InstallButton.Caption := 'Instalar / actualizar';
    InstallButton.ModalResult := mrYes;
    RemoveButton := TNewButton.Create(Form);
    RemoveButton.Parent := Form;
    RemoveButton.SetBounds(ScaleX(218), ScaleY(132), ScaleX(150), ScaleY(36));
    RemoveButton.Caption := 'Desinstalar';
    RemoveButton.ModalResult := mrNo;
    CancelButton := TNewButton.Create(Form);
    CancelButton.Parent := Form;
    CancelButton.SetBounds(ScaleX(378), ScaleY(132), ScaleX(138), ScaleY(36));
    CancelButton.Caption := 'Cancelar';
    CancelButton.Cancel := True;
    CancelButton.ModalResult := mrCancel;
    Result := Form.ShowModal;
  finally
    Form.Free;
  end;
end;

function InitializeSetup: Boolean;
var
  Action, ExitCode: Integer;
  AppDir, AppExe, Uninstaller: String;
begin
  Result := False;
  if ExpandConstant('{param:LUMA_ENGINE|0}') = '1' then begin
    Result := IsAdminInstallMode;
    if not Result then MsgBox('El motor de instalacion requiere /ALLUSERS.', mbError, MB_OK);
    Exit;
  end;
  AppDir := ExpandConstant('{commonpf64}\Luma Launcher');
  AppExe := AddBackslash(AppDir) + 'Luma.exe';
  Uninstaller := AddBackslash(AppDir) + 'unins000.exe';
  Action := ChooseAction;
  if Action = mrYes then begin
    Result := True;
  end else if Action = mrNo then begin
    try
      // Use a verified current helper outside every registered application
      // directory. It also supports portable and pre-Inno installations.
      AppExe := DownloadMaintenancePayload;
    except
      MsgBox('No se pudo preparar la desinstalacion: ' + GetExceptionMessage, mbError, MB_OK);
      Exit;
    end;
    if not ExecAsOriginalUser(AppExe, '--manage-uninstall-data', ExpandConstant('{tmp}'), SW_SHOWNORMAL, ewWaitUntilTerminated, ExitCode) then begin
      MsgBox('No se pudo abrir la confirmacion de datos. No se ha desinstalado Luma.', mbError, MB_OK);
      Exit;
    end;
    if ExitCode <> 0 then Exit;
    if not FileExists(Uninstaller) then begin
      try
        RemoveLegacyProgramFiles(AppDir);
        RegDeleteKeyIncludingSubkeys(HKLM, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Luma');
        RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Luma');
        MsgBox('Se retiraron las copias registradas de Luma y los datos que confirmaste.', mbInformation, MB_OK);
      except
        MsgBox(GetExceptionMessage, mbError, MB_OK);
      end;
      Exit;
    end;
    if not ShellExec('runas', Uninstaller, '', AppDir, SW_SHOWNORMAL, ewWaitUntilTerminated, ExitCode) then begin
      MsgBox('No se pudo abrir el desinstalador de la aplicacion.', mbError, MB_OK);
      Exit;
    end;
    if ExitCode = 0 then
      RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Luma');
  end;
end;

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

procedure ResolveRelease(var Url, Hash: String);
var
  Raw: AnsiString;
  Manifest, Version, ExpectedUrl: String;
  I: Integer;
begin
  DownloadTemporaryFile('https://github.com/Fernanboom34/Launcher/releases/download/{#ReleaseTag}/luma-release.json',
    'luma-release.json', '', @ManifestProgress);
  if not LoadStringFromFile(ExpandConstant('{tmp}\luma-release.json'), Raw) then
    RaiseException('No se pudo leer el manifiesto');
  Manifest := Raw;
  Version := JsonString(Manifest, 'version');
  if Version <> '{#SetupVersion}' then
    RaiseException('El manifiesto no corresponde a esta beta del instalador.');
  Url := JsonString(Manifest, 'url');
  Hash := JsonString(Manifest, 'sha256');
  ExpectedUrl := 'https://github.com/Fernanboom34/Launcher/releases/download/v' + Version + '/Luma-' + Version + '.exe';
  if Url <> ExpectedUrl then RaiseException('La descarga no pertenece al release esperado de Luma');
  if Length(Hash) <> 64 then RaiseException('SHA-256 no valido');
  for I := 1 to Length(Hash) do
    if Pos(Lowercase(Hash[I]), '0123456789abcdef') = 0 then RaiseException('SHA-256 no valido');
end;

function DownloadMaintenancePayload: String;
var
  Url, Hash: String;
begin
  Result := ExpandConstant('{tmp}\Luma.exe');
#ifdef FixturePayload
  if not FileCopy('{#FixturePayload}', Result, False) then RaiseException('No se pudo copiar la fixture');
#else
  ResolveRelease(Url, Hash);
  DownloadTemporaryFile(Url, 'Luma.exe', Hash, nil);
#endif
end;

procedure InitializeWizard;
begin
  DownloadPage := CreateDownloadPage('Descargando Luma', 'Se verificara la descarga antes de instalar.', nil);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Url, Hash: String;
begin
  Result := '';
#ifndef FixturePayload
  if CompareText(RemoveBackslashUnlessRoot(ExpandConstant('{app}')), RemoveBackslashUnlessRoot(ExpandConstant('{commonpf64}\Luma Launcher'))) <> 0 then begin
    Result := 'La instalacion debe usar Program Files\Luma Launcher.';
    Exit;
  end;
  DownloadPage.Show;
  try
    try
      ResolveRelease(Url, Hash);
      DownloadPage.Clear;
      DownloadPage.Add(Url, 'Luma.exe', Hash);
      DownloadPage.Download;
    except
      Result := GetExceptionMessage;
    end;
  finally
    DownloadPage.Hide;
  end;
#endif
end;
