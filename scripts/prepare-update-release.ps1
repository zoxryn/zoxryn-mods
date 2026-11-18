param(
    [Parameter(Mandatory=$true)][string]$Repository,
    [Parameter(Mandatory=$true)][string]$NotesFile,
    [string]$OutputDirectory
)
$ErrorActionPreference='Stop'
$project=Split-Path -Parent $PSScriptRoot
if($Repository -notmatch '^[A-Za-z0-9_-][A-Za-z0-9_.-]*/[A-Za-z0-9_-][A-Za-z0-9_.-]*$' -or $Repository -match '^(?i:owner|your-owner)/' -or $Repository -match '/(?i:repo|your-repo|repository)$') {throw 'A real GitHub OWNER/REPOSITORY is required. This script does not publish anything.'}
$version=(Get-Content -LiteralPath (Join-Path $project 'package.json') -Raw|ConvertFrom-Json).version
$channel=Get-Content -LiteralPath (Join-Path $project 'src-tauri\update-channel.json') -Raw|ConvertFrom-Json
if($channel.repository -ne $Repository){throw 'Configure the same repository with configure-update-channel.ps1, then rebuild the application before preparing the feed.'}
$artifact=Join-Path $project "src-tauri\target\release\bundle\nsis\Zoxryn Mods_${version}_x64-setup.exe"
if(!(Test-Path -LiteralPath ($artifact+'.sig'))){throw 'Build and sign the production installer first with scripts/build.ps1.'}
if(!$OutputDirectory){$OutputDirectory=Join-Path (Split-Path -Parent $project) "update-release-$version"}
[IO.Directory]::CreateDirectory($OutputDirectory)|Out-Null
$assetName="Zoxryn-Mods-$version-Setup.exe"
$outputArtifact=Join-Path $OutputDirectory $assetName
Copy-Item -LiteralPath $artifact -Destination $outputArtifact
Copy-Item -LiteralPath ($artifact+'.sig') -Destination ($outputArtifact+'.sig')
$python=Join-Path (Split-Path -Parent (Split-Path -Parent (Get-Command node).Source)) '..\python\python.exe'
$manifest=Join-Path $OutputDirectory 'latest.json'
& $python (Join-Path $PSScriptRoot 'make-update-manifest.py') --artifact $outputArtifact --version $version --url "https://github.com/$Repository/releases/download/v$version/$assetName" --output $manifest --notes (Get-Content -LiteralPath $NotesFile -Raw)
if($LASTEXITCODE){throw 'Manifest preparation failed'}
Push-Location $project
try{& (Join-Path $PSScriptRoot 'with-signing.ps1') -TauriArguments @('signer','sign','--app-version',$version,$manifest)}finally{Pop-Location}
Write-Output "Prepared signed release assets in $OutputDirectory. Nothing was uploaded or published. Upload the installer, installer.sig, latest.json and latest.json.sig together only after approval."
