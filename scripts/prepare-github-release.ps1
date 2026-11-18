param([Parameter(Mandatory=$true)][string]$NotesFile, [switch]$VerifyExistingBuild)
$ErrorActionPreference='Stop'
$project=Split-Path -Parent $PSScriptRoot
$workspace=Split-Path -Parent (Split-Path -Parent $project)
$version=(Get-Content (Join-Path $project 'package.json') -Raw|ConvertFrom-Json).version
$config=Get-Content (Join-Path $project 'src-tauri/tauri.conf.json') -Raw|ConvertFrom-Json
$cargoVersion=[regex]::Match((Get-Content (Join-Path $project 'src-tauri/Cargo.toml') -Raw),'(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if($version -notmatch '^\d+\.\d+\.\d+$' -or $version -ne $config.version -or $version -ne $cargoVersion){throw 'Stable version must agree in package.json, Cargo.toml and tauri.conf.json'}
if([version]$version -le [version]'0.1.6'){throw 'A GitHub-channel bootstrap must be newer than 0.1.6'}
$channel=Get-Content (Join-Path $project 'src-tauri/update-channel.json') -Raw|ConvertFrom-Json
$feed='https://github.com/zoxryn/zoxryn-mods/releases/latest/download/latest.json'
if($channel.repository -ne 'zoxryn/zoxryn-mods' -or $channel.feed_url -ne $feed -or @($config.plugins.updater.endpoints).Count -ne 1 -or $config.plugins.updater.endpoints[0] -ne $feed){throw 'Production channel configuration does not match the approved repository'}
if($config.identifier -ne 'com.zoxryn.mods' -or $config.mainBinaryName -ne 'zoxryn-mods' -or $config.bundle.windows.nsis.installMode -ne 'currentUser' -or !$config.plugins.updater.requireSignedVersion){throw 'Application identity, user-level install or updater trust configuration changed'}
$vault=Join-Path $env:LOCALAPPDATA 'ZoxrynModsSigning'
foreach($name in @('updater-release.key','updater-release.key.pub','updater-password.dpapi')){if(!(Test-Path -LiteralPath (Join-Path $vault $name))){throw 'The established signing identity is missing. Restore its original signing vault; no replacement was generated.'}}
$public=(Get-Content (Join-Path $project 'src-tauri/updater-public-key.pub') -Raw).Trim()
if($public -ne $config.plugins.updater.pubkey -or $public -ne (Get-Content (Join-Path $vault 'updater-release.key.pub') -Raw).Trim()){throw 'Established public signing identities disagree'}
if(!(Test-Path -LiteralPath $NotesFile -PathType Leaf)){throw 'A UTF-8 release-notes file is required'}
$notes=[IO.Path]::GetFullPath($NotesFile)
$toolchain=Join-Path $workspace 'work/toolchain'
if(Test-Path (Join-Path $toolchain 'cargo/bin')){$env:CARGO_HOME=Join-Path $toolchain 'cargo';$env:RUSTUP_HOME=Join-Path $toolchain 'rustup';$env:Path="$toolchain\cargo\bin;$toolchain\w64devkit\bin;$env:Path"}
$python=[IO.Path]::GetFullPath((Join-Path (Split-Path -Parent (Split-Path -Parent (Get-Command node).Source)) '../python/python.exe'))
Push-Location $project
try {
    & node scripts/audit-public-source.mjs
    if($LASTEXITCODE){throw 'Public-source safety audit failed'}
    if(!$VerifyExistingBuild){& ./scripts/build.ps1}
    if($VerifyExistingBuild){
        # Verification mode accepts only artifacts from the current tested sources.
        & node scripts/build-provenance.mjs --verify
        if($LASTEXITCODE){throw 'The current source/artifacts do not match the tested production build. Run the full workflow.'}
        $ui=Get-Content verification/interface-tests.json -Raw|ConvertFrom-Json
        if($ui.stats.unexpected -ne 0 -or $ui.stats.expected -lt 16){throw 'Passing frontend test evidence is required'}
    }
    & $python scripts/package_release.py
    if($LASTEXITCODE){throw 'Packaging failed'}
    & ./scripts/prepare-update-release.ps1 -Repository zoxryn/zoxryn-mods -NotesFile $notes
    $assets=Join-Path (Split-Path -Parent $project) "update-release-$version"
    $artifact=Join-Path $assets "Zoxryn-Mods-$version-Setup.exe"
    Push-Location src-tauri
    try {
        $verification=& cargo run --locked --quiet --example verify_release_signatures -- $artifact $version (Join-Path $assets 'latest.json')
        if($LASTEXITCODE){throw 'Offline artifact/manifest signature verification failed'}
    } finally {Pop-Location}
    $verification | Set-Content (Join-Path $assets 'signature-verification.json')
    Copy-Item -LiteralPath $notes -Destination (Join-Path $assets 'RELEASE_NOTES.md')
    $checksumNames=@("Zoxryn-Mods-$version-Setup.exe","Zoxryn-Mods-$version-Setup.exe.sig",'latest.json','latest.json.sig','RELEASE_NOTES.md')
    $lines=foreach($name in $checksumNames){$hash=(Get-FileHash -Algorithm SHA256 (Join-Path $assets $name)).Hash.ToLowerInvariant();"$hash  $name"}
    [IO.File]::WriteAllLines((Join-Path $assets 'SHA256SUMS.txt'),[string[]]$lines,[Text.UTF8Encoding]::new($false))
    @{version=$version;repository='zoxryn/zoxryn-mods';signaturesVerified=$true;preparedLocally=$true;readyToPublishLocally=$true;published=$false;onlineAvailabilityVerified=$false;publishingApprovalRequired=$true;bootstrapInstallationApprovalRequired=$true;existingBuildVerificationOnly=[bool]$VerifyExistingBuild} | ConvertTo-Json | Set-Content (Join-Path $assets 'readiness.json')
    Write-Output 'Signed GitHub release assets are prepared and verified locally. Publishing and bootstrap installation still require explicit approval.'
} finally {Pop-Location}
