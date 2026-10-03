param(
    [string]$InstallDir = (Join-Path ([Environment]::GetFolderPath("UserProfile")) ".algo\bin"),
    [switch]$KeepPath
)

$ErrorActionPreference = "Stop"

$algo = Join-Path $InstallDir "algo.exe"
if (Test-Path -LiteralPath $algo -PathType Leaf) {
    # Remove the native credential before deleting the executable that knows
    # its keyring service/user key. Fail closed: never claim a complete
    # uninstall if the operating-system vault could not be cleaned.
    & $algo logout
    if ($LASTEXITCODE -ne 0) {
        throw "Could not remove Algorithco account credentials; binaries were preserved"
    }
}

if (-not $KeepPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @($userPath -split ";" | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_) -and $_.TrimEnd("\") -ine $InstallDir.TrimEnd("\")
    })
    [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
}

foreach ($name in @("algo.exe", "algo-hook-client.exe", "algo-daemon.exe", "algo-tui.exe")) {
    $target = Join-Path $InstallDir $name
    if (Test-Path -LiteralPath $target -PathType Leaf) {
        Remove-Item -LiteralPath $target -Force
    }
}

Write-Output "Removed algo binaries from $InstallDir"
if (-not $KeepPath) { Write-Output "Removed $InstallDir from the user PATH" }
