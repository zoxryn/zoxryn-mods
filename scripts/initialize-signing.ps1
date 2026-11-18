param([switch]$NewSigningIdentity)
$ErrorActionPreference='Stop'
$project=Split-Path -Parent $PSScriptRoot
$vault=Join-Path $env:LOCALAPPDATA 'ZoxrynModsSigning'
$key=Join-Path $vault 'updater-release.key'
$passwordFile=Join-Path $vault 'updater-password.dpapi'
if((Get-Item -LiteralPath $vault -ErrorAction SilentlyContinue).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Signing vault cannot be a reparse point'}
[IO.Directory]::CreateDirectory($vault)|Out-Null
$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User
$acl=[Security.AccessControl.DirectorySecurity]::new()
$acl.SetOwner($sid);$acl.SetAccessRuleProtection($true,$false)
$rule=[Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow')
$acl.AddAccessRule($rule);Set-Acl -LiteralPath $vault -AclObject $acl
if(!(Test-Path -LiteralPath $key)){
    if((Test-Path -LiteralPath (Join-Path $project 'src-tauri\updater-public-key.pub')) -and !$NewSigningIdentity){throw 'An established public key exists. Restore its original signing vault; refusing to silently rotate the update identity. NewSigningIdentity is only for an intentionally separate distribution.'}
    if(Test-Path -LiteralPath $passwordFile){throw 'An incomplete signing vault exists; refusing to replace key material'}
    $plain=[Convert]::ToBase64String([Security.Cryptography.RandomNumberGenerator]::GetBytes(48))
    $secure=ConvertTo-SecureString $plain -AsPlainText -Force
    ConvertFrom-SecureString $secure | Set-Content -LiteralPath $passwordFile -NoNewline
    $capture=Join-Path $vault 'key-generation-private-output.tmp'
    try{
        & node (Join-Path $project 'node_modules\@tauri-apps\cli\tauri.js') signer generate --ci --password $plain --write-keys $key *> $capture
        if($LASTEXITCODE){throw 'Signing-key generation failed; secret output is suppressed'}
    }finally{$plain=$null;$secure=$null;if(Test-Path -LiteralPath $capture){Remove-Item -LiteralPath $capture -Force}}
}
if(!(Test-Path -LiteralPath $passwordFile)-or !(Test-Path -LiteralPath ($key+'.pub'))){throw 'Signing vault is incomplete; never regenerate an established release key'}
$public=(Get-Content -LiteralPath ($key+'.pub') -Raw).Trim()
[IO.File]::WriteAllText((Join-Path $project 'src-tauri\updater-public-key.pub'),$public,[Text.UTF8Encoding]::new($false))
$configPath=Join-Path $project 'src-tauri\tauri.conf.json'
$config=Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json -AsHashtable
$config.bundle.createUpdaterArtifacts=$true
if(!$config.ContainsKey('plugins')){$config.plugins=@{}}
if(!$config.plugins.ContainsKey('updater')){$config.plugins.updater=@{endpoints=@();windows=@{installMode='passive'}}}
$config.plugins.updater.pubkey=$public
$config.plugins.updater.requireSignedVersion=$true
[IO.File]::WriteAllText($configPath,($config|ConvertTo-Json -Depth 30),[Text.UTF8Encoding]::new($false))
Write-Output 'Release signing is configured. Private material is encrypted outside the project with a user-only Windows ACL; its password is protected with CurrentUser DPAPI.'
