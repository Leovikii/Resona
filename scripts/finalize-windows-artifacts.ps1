param(
    [string]$BundleDirectory = (Join-Path $PSScriptRoot "..\src-tauri\target\release\bundle\nsis"),
    [switch]$RequireUpdaterSignature
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
$finalizedCurrentVersion = @(
    $allInstallers | Where-Object {
        $_.Name -match "^Resona_$([regex]::Escape($version))_windows_(x64|x86|arm64)-setup\.exe$"
    }
)
if ($rawInstallers.Count -eq 1) {
    $installer = $rawInstallers[0]
}
elseif ($rawInstallers.Count -eq 0 -and $finalizedCurrentVersion.Count -eq 1) {
    $installer = $finalizedCurrentVersion[0]
}
else {
    throw "Expected exactly one current-version NSIS setup executable, found $($rawInstallers.Count) raw and $($finalizedCurrentVersion.Count) finalized"
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
$updaterArchitecture = switch ($architecture) {
    "arm64" { "aarch64" }
    "x86" { "i686" }
    default { "x86_64" }
}
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

if (Test-Path -LiteralPath $targetSignature -PathType Leaf) {
    $signature = (Get-Content -LiteralPath $targetSignature -Raw).Trim()
    if ([string]::IsNullOrWhiteSpace($signature)) {
        throw "Updater signature is empty: $targetSignature"
    }
    $releaseNotesPath = Join-Path $projectRoot "docs\releases\$version.md"
    $notes = if (Test-Path -LiteralPath $releaseNotesPath -PathType Leaf) {
        Get-Content -LiteralPath $releaseNotesPath -Raw -Encoding utf8
    }
    else {
        "Resona $version"
    }
    $downloadUrl = "https://github.com/Leovikii/Resona/releases/download/v$version/$([System.IO.Path]::GetFileName($target))"
    $platform = [ordered]@{
        url = $downloadUrl
        signature = $signature
    }
    $manifest = [ordered]@{
        version = $version
        notes = $notes.Trim()
        platforms = [ordered]@{
            "windows-$updaterArchitecture-nsis" = $platform
            "windows-$updaterArchitecture" = $platform
        }
    }
    $manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $resolvedBundle "latest.json") -Encoding utf8
}
elseif ($RequireUpdaterSignature) {
    throw "Updater signature is required but missing: $targetSignature"
}

Write-Output "Finalized $target"
Write-Output "SHA-256 $hash"
