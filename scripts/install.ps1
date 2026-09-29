param(
    [string]$SourceDir = (Join-Path $PSScriptRoot "..\agent\target\release"),
    [string]$InstallDir = (Join-Path ([Environment]::GetFolderPath("UserProfile")) ".algo\bin"),
    [switch]$SkipPath
)

$ErrorActionPreference = "Stop"
$source = (Resolve-Path -LiteralPath $SourceDir).Path
$required = @("algo.exe", "algo-hook-client.exe", "algo-daemon.exe")

foreach ($name in $required) {
    $candidate = Join-Path $source $name
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "Missing $candidate. Build first with: cargo build --release --manifest-path agent/Cargo.toml"
    }
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
foreach ($name in $required) {
    Copy-Item -LiteralPath (Join-Path $source $name) -Destination (Join-Path $InstallDir $name) -Force
}

if (-not $SkipPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @($userPath -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $alreadyPresent = $parts | Where-Object { $_.TrimEnd("\") -ieq $InstallDir.TrimEnd("\") }
    if (-not $alreadyPresent) {
        $updated = (@($parts) + $InstallDir) -join ";"
        [Environment]::SetEnvironmentVariable("Path", $updated, "User")
    }
    if (-not (($env:Path -split ";") | Where-Object { $_.TrimEnd("\") -ieq $InstallDir.TrimEnd("\") })) {
        $env:Path = "$InstallDir;$env:Path"
    }
}

& (Join-Path $InstallDir "algo.exe") --version
Write-Output "Installed algo and companion binaries in $InstallDir"
if (-not $SkipPath) {
    Write-Output "Added $InstallDir to the user PATH. Open a new terminal, then run: algo --help"
}
