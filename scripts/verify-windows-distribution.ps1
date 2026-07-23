param(
    [string]$BundleDirectory = (Join-Path $PSScriptRoot "..\src-tauri\target\release\bundle\nsis")
)

$ErrorActionPreference = "Stop"
$projectRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$package = Get-Content -LiteralPath (Join-Path $projectRoot "package.json") -Raw | ConvertFrom-Json
$config = Get-Content -LiteralPath (Join-Path $projectRoot "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
$cargoText = Get-Content -LiteralPath (Join-Path $projectRoot "src-tauri\Cargo.toml") -Raw
$ffmpegDependencyText = Get-Content -LiteralPath (Join-Path $projectRoot "src-tauri\src\ffmpeg_dependency.rs") -Raw
$versionMatch = [regex]::Match($cargoText, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
    throw "Cargo package version was not found"
}

$versions = @([string]$package.version, [string]$config.version, $versionMatch.Groups[1].Value)
if (@($versions | Select-Object -Unique).Count -ne 1) {
    throw "Package, Tauri and Cargo versions differ: $($versions -join ', ')"
}
if ($config.identifier -ne "io.github.vki.resona") {
    throw "Unexpected application identifier: $($config.identifier)"
}
if (-not $config.bundle.active -or @($config.bundle.targets).Count -ne 1 -or $config.bundle.targets[0] -ne "nsis") {
    throw "Windows distribution must enable only the NSIS bundle target"
}
if ($config.bundle.windows.nsis.installMode -ne "currentUser") {
    throw "NSIS must remain a non-elevated current-user installer"
}
if ($config.bundle.PSObject.Properties.Name -contains "externalBin") {
    throw "FFmpeg sidecars must not be bundled"
}
if (-not $ffmpegDependencyText.Contains("https://github.com/") -or $ffmpegDependencyText.Contains("gyan.dev")) {
    throw "FFmpeg runtime dependency must use the pinned GitHub Release asset, not a third-party site URL"
}

$resourceNames = @($config.bundle.resources.PSObject.Properties.Name)
if ($resourceNames | Where-Object { $_ -match '(?i)ffmpeg|ffprobe' }) {
    throw "FFmpeg binaries must not appear in bundle resources"
}
$extensions = @($config.bundle.fileAssociations | ForEach-Object { $_.ext } | ForEach-Object { $_ })
if (@($extensions | Sort-Object) -join "," -ne "flac,mp3,wav") {
    throw "Expected MP3, WAV and FLAC file associations"
}

$template = Join-Path $projectRoot "src-tauri\windows\nsis\installer.nsi"
$hooks = Join-Path $projectRoot "src-tauri\windows\nsis\installer-hooks.nsh"
$requiredFiles = @(
    $template,
    $hooks,
    (Join-Path $projectRoot "src-tauri\windows\nsis\header.bmp"),
    (Join-Path $projectRoot "src-tauri\windows\nsis\sidebar.bmp"),
    (Join-Path $projectRoot "src-tauri\icons\file-mp3.ico"),
    (Join-Path $projectRoot "src-tauri\icons\file-wav.ico"),
    (Join-Path $projectRoot "src-tauri\icons\file-flac.ico"),
    (Join-Path $projectRoot "src-tauri\icons\taskbar-previous.ico"),
    (Join-Path $projectRoot "src-tauri\icons\taskbar-play.ico"),
    (Join-Path $projectRoot "src-tauri\icons\taskbar-pause.ico"),
    (Join-Path $projectRoot "src-tauri\icons\taskbar-next.ico")
)
foreach ($path in $requiredFiles) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required distribution asset is missing: $path"
    }
}
$templateText = Get-Content -LiteralPath $template -Raw
$hookText = Get-Content -LiteralPath $hooks -Raw
if (-not $templateText.Contains('$LOCALAPPDATA\Programs\${PRODUCTNAME}')) {
    throw "The current-user install directory is not under LocalAppData\Programs"
}
if (-not $hookText.Contains('$APPDATA\io.github.vki.resona') -or -not $hookText.Contains('$LOCALAPPDATA\io.github.vki.resona')) {
    throw "Uninstall hooks must clean the exact roaming and local Resona data directories"
}

$resolvedBundle = [System.IO.Path]::GetFullPath($BundleDirectory)
$expectedBase = "Resona_$($package.version)_windows_x64-setup"
$installer = Join-Path $resolvedBundle "$expectedBase.exe"
$metadataPath = Join-Path $resolvedBundle "$expectedBase.json"
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw "Platform-qualified installer is missing: $installer"
}
if (-not (Test-Path -LiteralPath $metadataPath -PathType Leaf)) {
    throw "Installer metadata is missing: $metadataPath"
}
$installerInfo = Get-Item -LiteralPath $installer
if ($installerInfo.Length -gt 32MB) {
    throw "Installer is unexpectedly large ($($installerInfo.Length) bytes); check for bundled FFmpeg binaries"
}
$metadata = Get-Content -LiteralPath $metadataPath -Raw | ConvertFrom-Json
$actualHash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
if ($metadata.sha256 -ne $actualHash -or $metadata.file -ne $installerInfo.Name) {
    throw "Installer metadata does not match the finalized artifact"
}

Write-Output "Windows distribution verified"
Write-Output "Installer $($installerInfo.Name) ($($installerInfo.Length) bytes)"
Write-Output "SHA-256 $actualHash"
