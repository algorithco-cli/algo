param(
    [string]$InstallDir = (Join-Path ([Environment]::GetFolderPath("UserProfile")) ".algo\bin"),
    [switch]$KeepPath
)

$ErrorActionPreference = "Stop"

if (-not $KeepPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @($userPath -split ";" | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_) -and $_.TrimEnd("\") -ine $InstallDir.TrimEnd("\")
    })
    [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
}

foreach ($name in @("algo.exe", "algo-hook-client.exe", "algo-daemon.exe")) {
    $target = Join-Path $InstallDir $name
    if (Test-Path -LiteralPath $target -PathType Leaf) {
        Remove-Item -LiteralPath $target -Force
    }
}

Write-Output "Removed algo binaries from $InstallDir"
if (-not $KeepPath) { Write-Output "Removed $InstallDir from the user PATH" }
