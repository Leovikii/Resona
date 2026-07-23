param(
    [switch]$Force
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$archiveUri = "https://github.com/GyanD/codexffmpeg/releases/download/8.1.2/ffmpeg-8.1.2-essentials_build.zip"
$archiveHash = "DB580001CAA24AC104C8CB856CD113A87B0A443F7BDF47D8C12B1D740584A2EC"
$ffmpegHash = "1326DDE4C84FF1F96FE6B8916C5BED29E163E9B5DCCF995F6F3DB069D143EC5E"
$ffprobeHash = "B49CCC7C6547B141AD5A2F6EC69CC04323D7133D7704D70B331B904C63EECB07"

$repoRoot = Split-Path -Parent $PSScriptRoot
$binaryRoot = Join-Path $repoRoot "src-tauri/binaries"
$ffmpegTarget = Join-Path $binaryRoot "ffmpeg-x86_64-pc-windows-msvc.exe"
$ffprobeTarget = Join-Path $binaryRoot "ffprobe-x86_64-pc-windows-msvc.exe"

function Assert-FileHash {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Expected,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($actual -ne $Expected) {
        throw "$Label SHA-256 mismatch. Expected $Expected, received $actual."
    }
}

if (-not $Force -and
    (Test-Path -LiteralPath $ffmpegTarget -PathType Leaf) -and
    (Test-Path -LiteralPath $ffprobeTarget -PathType Leaf)) {
    Assert-FileHash -Path $ffmpegTarget -Expected $ffmpegHash -Label "ffmpeg"
    Assert-FileHash -Path $ffprobeTarget -Expected $ffprobeHash -Label "ffprobe"
    Write-Host "FFmpeg test tools are present and verified."
    exit 0
}

$tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$tempRoot = Join-Path $tempBase ("resona-ffmpeg-" + [Guid]::NewGuid().ToString("N"))
$archivePath = Join-Path $tempRoot "ffmpeg.zip"
$extractRoot = Join-Path $tempRoot "extract"

try {
    New-Item -ItemType Directory -Path $extractRoot -Force | Out-Null
    Write-Host "Downloading pinned FFmpeg 8.1.2 essentials build..."
    Invoke-WebRequest -Uri $archiveUri -OutFile $archivePath -UseBasicParsing
    Assert-FileHash -Path $archivePath -Expected $archiveHash -Label "FFmpeg archive"

    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractRoot
    $ffmpegSource = Get-ChildItem -LiteralPath $extractRoot -Recurse -File -Filter "ffmpeg.exe" | Select-Object -First 1
    $ffprobeSource = Get-ChildItem -LiteralPath $extractRoot -Recurse -File -Filter "ffprobe.exe" | Select-Object -First 1
    if ($null -eq $ffmpegSource -or $null -eq $ffprobeSource) {
        throw "The verified FFmpeg archive does not contain ffmpeg.exe and ffprobe.exe."
    }

    Assert-FileHash -Path $ffmpegSource.FullName -Expected $ffmpegHash -Label "ffmpeg"
    Assert-FileHash -Path $ffprobeSource.FullName -Expected $ffprobeHash -Label "ffprobe"

    New-Item -ItemType Directory -Path $binaryRoot -Force | Out-Null
    Copy-Item -LiteralPath $ffmpegSource.FullName -Destination $ffmpegTarget -Force
    Copy-Item -LiteralPath $ffprobeSource.FullName -Destination $ffprobeTarget -Force
    Assert-FileHash -Path $ffmpegTarget -Expected $ffmpegHash -Label "installed ffmpeg"
    Assert-FileHash -Path $ffprobeTarget -Expected $ffprobeHash -Label "installed ffprobe"
    Write-Host "FFmpeg test tools downloaded and verified."
}
finally {
    $resolvedTempRoot = [IO.Path]::GetFullPath($tempRoot)
    $tempLeaf = Split-Path -Leaf $resolvedTempRoot
    if ($resolvedTempRoot.StartsWith($tempBase, [StringComparison]::OrdinalIgnoreCase) -and
        $tempLeaf.StartsWith("resona-ffmpeg-", [StringComparison]::Ordinal)) {
        Remove-Item -LiteralPath $resolvedTempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
