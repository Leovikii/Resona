param(
    [string]$BundleDirectory = (Join-Path $PSScriptRoot "..\src-tauri\target\release\bundle\nsis")
)

$ErrorActionPreference = "Stop"
$projectRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$package = Get-Content -LiteralPath (Join-Path $projectRoot "package.json") -Raw | ConvertFrom-Json
$version = [string]$package.version
$resolvedBundle = [System.IO.Path]::GetFullPath($BundleDirectory)

if (-not (Test-Path -LiteralPath $resolvedBundle -PathType Container)) {
    throw "NSIS bundle directory does not exist: $resolvedBundle"
}

$allInstallers = @(Get-ChildItem -LiteralPath $resolvedBundle -File -Filter "*-setup.exe")
$rawInstallers = @($allInstallers | Where-Object { $_.Name -notmatch "_windows_(x64|x86|arm64)-setup\.exe$" })
if ($rawInstallers.Count -eq 1) {
    $installer = $rawInstallers[0]
}
elseif ($rawInstallers.Count -eq 0 -and $allInstallers.Count -eq 1) {
    $installer = $allInstallers[0]
}
else {
    throw "Expected exactly one new NSIS setup executable, found $($rawInstallers.Count) raw and $($allInstallers.Count) total"
}

$architecture = if ($installer.Name -match "_(windows_)?arm64-setup\.exe$") {
    "arm64"
}
elseif ($installer.Name -match "_(windows_)?x86-setup\.exe$") {
    "x86"
}
else {
    "x64"
}

$baseName = "Resona_${version}_windows_${architecture}-setup"
$target = Join-Path $resolvedBundle "$baseName.exe"
if ($installer.FullName -ne $target) {
    Move-Item -LiteralPath $installer.FullName -Destination $target -Force
}

$sourceSignature = "$($installer.FullName).sig"
$targetSignature = "$target.sig"
if (Test-Path -LiteralPath $sourceSignature -PathType Leaf) {
    if ($sourceSignature -ne $targetSignature) {
        Move-Item -LiteralPath $sourceSignature -Destination $targetSignature -Force
    }
}

$hash = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash
$metadata = [ordered]@{
    product = "Resona"
    version = $version
    platform = "windows"
    architecture = $architecture
    file = [System.IO.Path]::GetFileName($target)
    sha256 = $hash
    signed = Test-Path -LiteralPath $targetSignature -PathType Leaf
}
$metadataPath = Join-Path $resolvedBundle "$baseName.json"
$metadata | ConvertTo-Json | Set-Content -LiteralPath $metadataPath -Encoding utf8

Write-Output "Finalized $target"
Write-Output "SHA-256 $hash"
