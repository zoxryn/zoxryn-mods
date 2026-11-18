param([Parameter(Mandatory=$true)][string]$Repository)
$ErrorActionPreference='Stop'
if($Repository -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' -or $Repository -match '^(OWNER|YOUR-OWNER)/' -or $Repository -match '/(REPO|YOUR-REPO)$'){throw 'Supply the real GitHub OWNER/REPOSITORY. Placeholder feeds cannot be activated.'}
$project=Split-Path -Parent $PSScriptRoot
$feed="https://github.com/$Repository/releases/latest/download/latest.json"
$channel=@{repository=$Repository;feed_url=$feed;recovery_url="https://github.com/$Repository/releases/latest"}
$channel|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $project 'src-tauri\update-channel.json')
$path=Join-Path $project 'src-tauri\tauri.conf.json'
$config=Get-Content -LiteralPath $path -Raw|ConvertFrom-Json -AsHashtable
$config.plugins.updater.endpoints=@($feed)
$config|ConvertTo-Json -Depth 30|Set-Content -LiteralPath $path
Write-Output 'The public channel is configured for the next build. Nothing was deployed or published.'
