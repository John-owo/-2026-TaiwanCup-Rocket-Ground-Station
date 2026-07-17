$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $repo

function Assert-LastExitCode([string]$label) {
    if ($LASTEXITCODE -ne 0) {
        throw "$label failed with exit code $LASTEXITCODE"
    }
    Write-Host "[PASS] $label"
}

$tauriConfigPath = Join-Path $repo 'src-tauri\tauri.conf.json'
$tauriConfig = Get-Content -Raw -Encoding UTF8 $tauriConfigPath | ConvertFrom-Json
$version = [string]$tauriConfig.version
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Tauri version must be semantic versioning, got '$version'"
}

# Use the bundled workspace runtime when available so future packaging does not
# depend on a developer's global Node or pnpm installation.
$bundledNode = 'C:\Users\wu970\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin'
$bundledOverride = 'C:\Users\wu970\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin\override'
$bundledFallback = 'C:\Users\wu970\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin\fallback'
$runtimeEntries = @($bundledNode, $bundledOverride, $bundledFallback) | Where-Object {
    Test-Path -LiteralPath $_
}
if ($runtimeEntries.Count -gt 0) {
    $env:PATH = (($runtimeEntries -join ';') + ';' + $env:PATH)
}

$pnpm = (Get-Command pnpm.cmd -ErrorAction SilentlyContinue).Source
if (-not $pnpm) {
    $pnpm = Join-Path $bundledFallback 'pnpm.cmd'
}
if (-not (Test-Path -LiteralPath $pnpm)) {
    throw 'pnpm.cmd not found; install pnpm or load the bundled workspace runtime'
}

$tauriCli = Join-Path $repo 'src-ui\node_modules\.bin\tauri.CMD'
if (-not (Test-Path -LiteralPath $tauriCli)) {
    throw "Tauri CLI not found at $tauriCli; run pnpm install in src-ui first"
}

Push-Location (Join-Path $repo 'src-tauri')
try {
    & cargo test
    Assert-LastExitCode 'ground station Rust tests'
    & cargo check
    Assert-LastExitCode 'ground station Rust check'
} finally {
    Pop-Location
}

Push-Location (Join-Path $repo 'src-ui')
try {
    & $pnpm test
    Assert-LastExitCode 'ground station frontend tests'
    & $pnpm check
    Assert-LastExitCode 'ground station Svelte and TypeScript checks'
    & $pnpm build
    Assert-LastExitCode 'ground station frontend production build'
} finally {
    Pop-Location
}

& $tauriCli build --no-bundle --config '.\src-tauri\tauri.workspace-build.json'
Assert-LastExitCode 'ground station Tauri release build'

$sourceExe = Join-Path $repo 'src-tauri\target\release\app.exe'
if (-not (Test-Path -LiteralPath $sourceExe)) {
    throw "Tauri build completed but release executable was not found at $sourceExe"
}

$artifactDir = Join-Path $repo 'artifacts'
New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
$buildStamp = Get-Date -Format 'yyyy-MM-dd_HHmmss'
$artifactStem = "GroundStation_${version}_Portable_${buildStamp}"
$artifactPath = Join-Path $artifactDir ($artifactStem + '.exe')
Copy-Item -LiteralPath $sourceExe -Destination $artifactPath -Force

$manifest = [ordered]@{
    productName = [string]$tauriConfig.productName
    version = $version
    builtAt = (Get-Date).ToString('o')
    executable = (Split-Path -Leaf $artifactPath)
    source = 'src-tauri/target/release/app.exe'
    validation = @(
        'cargo test',
        'cargo check',
        'pnpm test',
        'pnpm check',
        'pnpm build',
        'tauri build --no-bundle'
    )
}
$manifestPath = Join-Path $artifactDir ($artifactStem + '.json')
$manifest | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 -LiteralPath $manifestPath

$latestPath = Join-Path $artifactDir 'LATEST.txt'
@(
    "version=$version"
    "executable=$(Split-Path -Leaf $artifactPath)"
    "manifest=$(Split-Path -Leaf $manifestPath)"
) | Set-Content -Encoding UTF8 -LiteralPath $latestPath

Write-Host "[PASS] packaged $artifactPath"
Write-Host "[PASS] manifest $manifestPath"
