$ErrorActionPreference='Stop'
if ($env:APPROVED_COMMIT -cnotmatch '^[0-9a-f]{40}$' -or $env:RELEASE_VERSION -cnotmatch '^\d+\.\d+\.\d+$') {throw 'Explicit immutable release inputs required'}
if ((git rev-parse HEAD) -ne $env:APPROVED_COMMIT) {throw 'Checkout differs from approved source'}
if ((Get-Content package.json -Raw|ConvertFrom-Json).version -ne $env:RELEASE_VERSION) {throw 'Version mismatch'}
if ($env:TAURI_SIGNING_PRIVATE_KEY -or $env:TAURI_SIGNING_PRIVATE_KEY_PATH -or $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {throw 'Unsigned build worker must not have production signing secrets'}
function Check([string]$Message) {if ($LASTEXITCODE) {throw $Message}}
pnpm install --frozen-lockfile; Check 'Locked dependency install failed'
node scripts/audit-public-source.mjs; Check 'Public source audit failed'
node --test tests/public-source-audit.test.mjs; Check 'Audit regression failed'
python -m unittest discover -s tests -p 'release_automation_test.py'; Check 'Release policy tests failed'
python scripts/check_advisories.py --enforce-windows; Check 'Windows Rust dependency advisory gate failed'
pnpm audit --audit-level high; Check 'Dependency audit failed'
pnpm build; Check 'Frontend build failed'
pnpm test:ui; Check 'Frontend tests failed'
Push-Location src-tauri
try {
    cargo test --locked; Check 'Backend regression failed'
    cargo test --locked --features desktop --bin zoxryn-mods; Check 'Native updater tests failed'
} finally {Pop-Location}
$env:CARGO_ENCODED_RUSTFLAGS=@("--remap-path-prefix=$pwd=/zoxryn-build","--remap-path-prefix=$env:USERPROFILE=/build-user") -join [char]0x1f
$config=Join-Path $env:RUNNER_TEMP 'unsigned-production.json'
[IO.File]::WriteAllText($config,'{"bundle":{"createUpdaterArtifacts":false}}')
node node_modules/@tauri-apps/cli/tauri.js build --features desktop --config $config; Check 'Unsigned Windows build failed'
python scripts/release/candidate.py; Check 'Candidate validation failed'
