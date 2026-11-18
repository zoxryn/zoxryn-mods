param([Parameter(Mandatory=$true)][string[]]$TauriArguments)
$ErrorActionPreference='Stop'
$project=Split-Path -Parent $PSScriptRoot
$vault=Join-Path $env:LOCALAPPDATA 'ZoxrynModsSigning'
$secure=Get-Content -LiteralPath (Join-Path $vault 'updater-password.dpapi') -Raw | ConvertTo-SecureString
$credential=[PSCredential]::new('signing',$secure)
$env:TAURI_SIGNING_PRIVATE_KEY_PATH=Join-Path $vault 'updater-release.key'
# Tauri build accepts a path in KEY; signer sign uses the separate PATH option.
if($TauriArguments[0] -ne 'signer'){$env:TAURI_SIGNING_PRIVATE_KEY=$env:TAURI_SIGNING_PRIVATE_KEY_PATH}
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$credential.GetNetworkCredential().Password
try{
    & node (Join-Path $project 'node_modules\@tauri-apps\cli\tauri.js') @TauriArguments
    if($LASTEXITCODE){throw "Tauri signing/build command failed (exit $LASTEXITCODE)"}
}finally{
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    $credential=$null;$secure=$null
}
