param([switch]$TestsOnly)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$workspace = Split-Path -Parent (Split-Path -Parent $project)
$toolchain = Join-Path $workspace 'work\toolchain'
if (Test-Path (Join-Path $toolchain 'cargo\bin')) {
    $env:CARGO_HOME = Join-Path $toolchain 'cargo'
    $env:RUSTUP_HOME = Join-Path $toolchain 'rustup'
    $env:Path = "$toolchain\cargo\bin;$toolchain\w64devkit\bin;$env:Path"
}
Push-Location $project
try {
    Push-Location (Join-Path $project 'src-tauri')
    try { cargo test --locked } finally { Pop-Location }
    if ($LASTEXITCODE) { throw 'Backend tests failed' }
    if (!$TestsOnly) {
        & node --test (Join-Path $project 'tests/public-source-audit.test.mjs')
        if($LASTEXITCODE){throw 'Public-source audit regression failed'}
        $pnpm = Get-Command pnpm -ErrorAction SilentlyContinue
        $pnpmScript = Join-Path (Split-Path -Parent (Split-Path -Parent (Get-Command node).Source)) 'node_modules\pnpm\bin\pnpm.cjs'
        function Invoke-Pnpm {
            if ($pnpm) { & $pnpm.Source @args }
            elseif (Test-Path -LiteralPath $pnpmScript) { & node $pnpmScript @args }
            else { throw 'Install pnpm (https://pnpm.io/installation), then run this build script again.' }
        }
        Invoke-Pnpm install --frozen-lockfile
        if ($LASTEXITCODE) { throw 'Dependency installation failed' }
        Invoke-Pnpm build
        if ($LASTEXITCODE) { throw 'Frontend build failed' }
        $previousBrowser=$env:ZOXRYN_TEST_BROWSER
        try {
            if(!$env:ZOXRYN_TEST_BROWSER -and $env:OS -eq 'Windows_NT'){$env:ZOXRYN_TEST_BROWSER='msedge'}
            Invoke-Pnpm test:ui
            if($LASTEXITCODE){throw 'Frontend tests failed'}
        } finally { $env:ZOXRYN_TEST_BROWSER=$previousBrowser }
        Push-Location (Join-Path $project 'src-tauri')
        try { cargo test --locked --features desktop --bin zoxryn-mods } finally { Pop-Location }
        if($LASTEXITCODE){throw 'Desktop updater tests failed'}
        $previousFlags=$env:CARGO_ENCODED_RUSTFLAGS
        try {
            $flags=@("--remap-path-prefix=$workspace=/zoxryn-build", "--remap-path-prefix=$env:USERPROFILE=/user")
            if((rustc -vV | Select-String 'host:.*windows-gnu')){$flags=@('-C','link-self-contained=yes')+$flags}
            $env:CARGO_ENCODED_RUSTFLAGS=$flags -join [char]0x1f
            & (Join-Path $PSScriptRoot 'with-signing.ps1') -TauriArguments @('build', '--features', 'desktop')
            if ($LASTEXITCODE) { throw 'Desktop build failed' }
        } finally { $env:CARGO_ENCODED_RUSTFLAGS=$previousFlags }
        & node (Join-Path $PSScriptRoot 'build-provenance.mjs')
        if($LASTEXITCODE){throw 'Build provenance or binary privacy verification failed'}
    }
} finally { Pop-Location }
