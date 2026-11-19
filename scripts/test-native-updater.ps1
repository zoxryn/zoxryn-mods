param([string]$Root,[string]$FromVersion,[string]$ToVersion)
$ErrorActionPreference='Stop'
$project=Split-Path -Parent $PSScriptRoot
$workspace=Split-Path -Parent (Split-Path -Parent $project)
if(!$Root){$Root=Join-Path $workspace 'work/github-updater/native-qa'}
$Root=[IO.Path]::GetFullPath($Root)
$toolchain=Join-Path $workspace 'work/toolchain'
$env:CARGO_HOME=Join-Path $toolchain 'cargo';$env:RUSTUP_HOME=Join-Path $toolchain 'rustup';$env:Path="$toolchain\cargo\bin;$toolchain\w64devkit\bin;$env:Path"
$python=[IO.Path]::GetFullPath((Join-Path (Split-Path -Parent (Split-Path -Parent (Get-Command node).Source)) '../python/python.exe'))
$previousTarget=$env:CARGO_TARGET_DIR
$previousRoot=$env:ZOXRYN_UPDATER_QA_ROOT
$from=if($FromVersion){$FromVersion}else{(Get-Content (Join-Path $project 'package.json') -Raw|ConvertFrom-Json).version}
$parts=[version]$from
$to=if($ToVersion){$ToVersion}else{"$($parts.Major).$($parts.Minor).$($parts.Build+1)"}
if($from -notmatch '^\d+\.\d+\.\d+$' -or $to -notmatch '^\d+\.\d+\.\d+$' -or [version]$to -le [version]$from){throw 'Stable increasing QA versions required'}
$installed=$false
$ownPsPid=[Diagnostics.Process]::GetCurrentProcess().Id
Push-Location $project
try {
    & ./scripts/prepare-updater-qa.ps1 -Root $Root -FromVersion $from -ToVersion $to
    $env:ZOXRYN_UPDATER_QA_ROOT=$Root
    # Production artifacts never share this target directory.
    $env:CARGO_TARGET_DIR=Join-Path $workspace 'work/github-updater/qa-build-cache'
    foreach($version in @($from,$to)) {
        & ./scripts/with-signing.ps1 -TauriArguments @('build','--features','updater-qa','--config',(Join-Path $Root "tauri-$version.json"),'--','--bin','zoxryn-mods-updater-qa')
        $artifact=Join-Path $env:CARGO_TARGET_DIR "release/bundle/nsis/Zoxryn Mods Updater QA_${version}_x64-setup.exe"
        if(!(Test-Path -LiteralPath $artifact)){throw 'A dedicated QA installer was not generated; production installers must never be substituted'}
        $name=if($version -eq $from){'source.exe'}else{'candidate.exe'}
        Copy-Item -LiteralPath $artifact -Destination (Join-Path $Root $name)
        Copy-Item -LiteralPath ($artifact+'.sig') -Destination (Join-Path $Root ($name+'.sig'))
        Push-Location src-tauri
        try { cargo run --locked --quiet --example verify_release_signatures -- (Join-Path $Root $name) $version; if($LASTEXITCODE){throw 'QA artifact signature or version verification failed'} } finally {Pop-Location}
    }
    $port=(Get-Content (Join-Path $Root 'port.txt')).Trim()
    & $python scripts/make-update-manifest.py --artifact (Join-Path $Root 'candidate.exe') --version $to --url "http://127.0.0.1:$port/installer.exe" --output (Join-Path $Root 'latest.json') --notes 'QA signed application update'
    if($LASTEXITCODE){throw 'QA manifest preparation failed'}
    & ./scripts/with-signing.ps1 -TauriArguments @('signer','sign','--app-version',$to,(Join-Path $Root 'latest.json'))
    & $python scripts/make-updater-qa-variants.py $Root
    if($LASTEXITCODE){throw 'Negative QA fixture generation failed'}
    foreach($name in @('expired','downgrade','prerelease','invalid','wrong-artifact-version')) {
        $version=if($name -eq 'downgrade'){'0.1.6'}elseif($name -eq 'prerelease'){"$to-beta.1"}else{$to}
        & ./scripts/with-signing.ps1 -TauriArguments @('signer','sign','--app-version',$version,(Join-Path $Root ($name+'.json')))
    }
    & $python scripts/verify-updater-qa.py capture $Root
    if($LASTEXITCODE){throw 'Synthetic baseline could not be verified'}
    $install=Join-Path $Root 'Installed QA App'
    $setup=Start-Process -FilePath (Join-Path $Root 'source.exe') -ArgumentList @('/S',"/D=$install") -PassThru -Wait -WindowStyle Hidden
    if($setup.ExitCode -ne 0){throw 'Isolated QA installation failed'}
    $installed=$true
    $binary=Join-Path $install 'zoxryn-mods-updater-qa.exe'
    if(!(Test-Path -LiteralPath $binary)){throw 'QA executable was not installed'}
    $app=Start-Process -FilePath $binary -PassThru -WindowStyle Hidden
    $deadline=(Get-Date).AddSeconds(150)
    while((Get-Date) -lt $deadline -and !(Test-Path (Join-Path $Root 'evidence/relaunch.json')) -and !(Test-Path (Join-Path $Root 'evidence/failed.json'))){Start-Sleep -Milliseconds 500}
    if(!(Test-Path (Join-Path $Root 'evidence/relaunch.json'))){throw 'Native upgrade/restart did not finish; retained QA evidence contains the failure'}
    & $python scripts/verify-updater-qa.py verify $Root
    if($LASTEXITCODE){throw 'Native updater data/progress verification failed'}
    Write-Output 'Real signed upgrade, restart, negative scenarios, data preservation and native progress checks passed in the isolated QA installation.'
} finally {
    if(Test-Path (Join-Path $Root 'server-pid.txt')) {
        $serverId=[int](Get-Content (Join-Path $Root 'server-pid.txt'))
        $server=Get-CimInstance Win32_Process -Filter ("ProcessId = $serverId")
        if($server -and $server.ParentProcessId -eq $ownPsPid -and $server.CommandLine.Contains('updater-qa-server.py') -and $server.CommandLine.Contains($Root)){Stop-Process -Id $serverId}
    }
    if($installed) {
        $qaInstall=Join-Path $Root 'Installed QA App'
        foreach($process in Get-CimInstance Win32_Process -Filter "Name = 'zoxryn-mods-updater-qa.exe'") {
            if($process.ExecutablePath -eq (Join-Path $qaInstall 'zoxryn-mods-updater-qa.exe')){Stop-Process -Id $process.ProcessId}
        }
        $uninstaller=Join-Path $qaInstall 'uninstall.exe'
        if(Test-Path -LiteralPath $uninstaller){$uninstall=Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru -WindowStyle Hidden; if($uninstall.ExitCode -ne 0){Write-Warning 'QA uninstall did not complete; its isolated directory has been retained'}}
    }
    $env:CARGO_TARGET_DIR=$previousTarget;$env:ZOXRYN_UPDATER_QA_ROOT=$previousRoot
    Pop-Location
}
