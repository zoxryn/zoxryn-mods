param([Parameter(Mandatory)][string]$Policy,[Parameter(Mandatory)][string]$Artifact,[Parameter(Mandatory)][string]$Version)
$ErrorActionPreference='Stop'
$p=Get-Content -LiteralPath $Policy -Raw|ConvertFrom-Json
if (!$p.enabled -or $env:COMPUTERNAME -ne $p.host) {throw 'Signing environment is not activated'}
$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
if ($sid -ne $p.user_sid) {throw 'Dedicated signing identity mismatch'}
$writeRights=[Security.AccessControl.FileSystemRights]::Write -bor [Security.AccessControl.FileSystemRights]::Delete -bor [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
foreach($path in @($p.control_root,$p.vault,$Policy,$p.tools.signer.path,$PSCommandPath,(Join-Path $p.vault 'updater-release.key'),(Join-Path $p.vault 'updater-password.dpapi'),(Join-Path $p.vault 'updater-release.key.pub'))) {
    $item=Get-Item -LiteralPath $path
    $ancestor=$item
    while($ancestor){if($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Signing paths cannot contain reparse points'};$ancestor=if($ancestor -is [IO.FileInfo]){$ancestor.Directory}else{$ancestor.Parent}}
    $acl=Get-Acl -LiteralPath $path
    if (!$acl.AreAccessRulesProtected) {throw 'Explicit protected signing ACLs are required'}
    if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin @($sid,'S-1-5-18','S-1-5-32-544')) {throw 'Unexpected signing path owner'}
    foreach($rule in $acl.Access) {
        $identity=$rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
        if ($rule.AccessControlType -eq 'Allow' -and ($rule.FileSystemRights -band $writeRights) -and $identity -notin @($sid,'S-1-5-18','S-1-5-32-544')) {throw 'Unexpected principal has signing write access'}
    }
}
$tool=$p.tools.signer.path
if ((Get-FileHash -LiteralPath $tool -Algorithm SHA256).Hash.ToLowerInvariant() -ne $p.tools.signer.sha256) {throw 'Pinned signer changed'}
if ($Version -cnotmatch '^\d+\.\d+\.\d+$' -or [IO.Path]::GetExtension($Artifact) -notin @('.exe','.json')) {throw 'Signing request invalid'}
$candidate=[IO.Path]::GetFullPath($Artifact)
if (!$candidate.StartsWith([IO.Path]::GetFullPath($p.work_root).TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)) {throw 'Signing input escaped dedicated release workspace'}
if ((Get-Content (Join-Path $p.vault 'updater-release.key.pub') -Raw).Trim() -ne $p.public_key) {throw 'Established updater identity mismatch'}
$secure=Get-Content (Join-Path $p.vault 'updater-password.dpapi') -Raw|ConvertTo-SecureString
$credential=[PSCredential]::new('release-signing',$secure)
$prior=Get-Location
try {
    Set-Location -LiteralPath $p.control_root
    $env:TAURI_SIGNING_PRIVATE_KEY_PATH=Join-Path $p.vault 'updater-release.key'
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$credential.GetNetworkCredential().Password
    # The pinned official native CLI receives the secret only through its environment.
    # No npm, Cargo build, checkout script, downloaded executable or plugin is run here.
    & $tool signer sign --app-version $Version $candidate 2>&1|Out-Null
    if ($LASTEXITCODE) {throw 'Trusted signer failed; its output was suppressed'}
} finally {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    $credential=$null;$secure=$null
    Set-Location -LiteralPath $prior
}
