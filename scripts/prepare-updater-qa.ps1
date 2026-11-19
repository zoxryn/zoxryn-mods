param([string]$Root, [string]$FromVersion='0.1.7', [string]$ToVersion='0.1.8')
$ErrorActionPreference='Stop'
$project=Split-Path -Parent $PSScriptRoot
$workspace=Split-Path -Parent (Split-Path -Parent $project)
if(!$Root){$Root=Join-Path $workspace 'work\github-updater\native-qa'}
$root=[IO.Path]::GetFullPath($Root)
if(!$root.StartsWith([IO.Path]::GetFullPath((Join-Path $workspace 'work'))+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'QA root must be a new directory inside this workspace work directory'}
if(Test-Path -LiteralPath $root){throw 'Updater QA directory exists; preserve evidence and choose a fresh test root.'}
$toolchain=Join-Path $workspace 'work\toolchain'
$env:CARGO_HOME=Join-Path $toolchain 'cargo';$env:RUSTUP_HOME=Join-Path $toolchain 'rustup';$env:Path="$env:CARGO_HOME\bin;$toolchain\w64devkit\bin;$env:Path"
Push-Location (Join-Path $project 'src-tauri')
try{cargo run --example create_synthetic_workspace -- $root;if($LASTEXITCODE){throw 'Synthetic fixture creation failed'}}finally{Pop-Location}
Set-Content -LiteralPath (Join-Path $root 'SYNTHETIC-UPDATER-QA') -Value 'ZOXRYN-SYNTHETIC-ONLY'
Set-Content -LiteralPath (Join-Path $root 'scenario.txt') -Value 'valid'
@{from=$FromVersion;to=$ToVersion}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $root 'versions.json')
New-Item -ItemType Directory -Path (Join-Path $root 'webview') | Out-Null
$python=Join-Path (Split-Path -Parent (Split-Path -Parent (Get-Command node).Source)) '..\python\python.exe'
$python=[IO.Path]::GetFullPath($python)
$server=Start-Process -FilePath $python -ArgumentList @(('"'+(Join-Path $PSScriptRoot 'updater-qa-server.py')+'"'),('"'+$root+'"')) -WindowStyle Hidden -PassThru
$server.Id|Set-Content -LiteralPath (Join-Path $root 'server-pid.txt')
for($i=0;$i -lt 50 -and !(Test-Path -LiteralPath (Join-Path $root 'port.txt'));$i++){Start-Sleep -Milliseconds 100}
$port=(Get-Content -LiteralPath (Join-Path $root 'port.txt')).Trim()
@{repository=$null;feed_url="http://127.0.0.1:$port/latest.json";recovery_url=$null}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $root 'channel.json')
$key=(Get-Content -LiteralPath (Join-Path $project 'src-tauri\updater-public-key.pub') -Raw).Trim()
$hook=Join-Path $root 'installer-hooks.nsh'
@'
!macro NSIS_HOOK_PREUNINSTALL
  StrCpy $DeleteAppDataCheckboxState 0
!macroend
'@|Set-Content -LiteralPath $hook
foreach($version in @($FromVersion,$ToVersion)){
 $config=@{productName='Zoxryn Mods Updater QA';mainBinaryName='zoxryn-mods-updater-qa';version=$version;identifier='com.zoxryn.mods.updater-qa';app=@{windows=@(@{label='main';title='Zoxryn Mods Updater QA';url='qa.html';width=900;height=700;visible=$false;dataDirectory=(Join-Path $root 'webview')})};bundle=@{createUpdaterArtifacts=$true;windows=@{nsis=@{installMode='currentUser';startMenuFolder='Zoxryn Mods Updater QA';installerHooks=$hook}}};plugins=@{updater=@{pubkey=$key;endpoints=@();requireSignedVersion=$true;dangerousInsecureTransportProtocol=$true;windows=@{installMode='quiet'}}}}
 $config|ConvertTo-Json -Depth 30|Set-Content -LiteralPath (Join-Path $root "tauri-$version.json")
}
Write-Output 'Synthetic updater fixture and loopback-only test server are ready. Real application registration and data are not used.'
