param(
    [string]$OutputDirectory = (Join-Path $PSScriptRoot "..\tests\fixtures\audio")
)

$ErrorActionPreference = "Stop"

$output = [System.IO.Path]::GetFullPath($OutputDirectory)
$workspace = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if (-not $output.StartsWith($workspace, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Fixture output must remain inside the Resona workspace: $output"
}

$ffmpeg = (Get-Command ffmpeg -ErrorAction Stop).Source
$flac = (Get-Command flac -ErrorAction Stop).Source
New-Item -ItemType Directory -Force -Path $output | Out-Null

$common = @(
    "-hide_banner", "-loglevel", "error", "-y",
    "-fflags", "+bitexact", "-flags:a", "+bitexact",
    "-f", "lavfi", "-i", "sine=frequency=997:duration=0.35",
    "-map_metadata", "-1", "-ac", "2"
)

function Invoke-Encoder {
    param(
        [string]$Name,
        [string[]]$Arguments
    )

    & $ffmpeg @common @Arguments (Join-Path $output $Name)
    if ($LASTEXITCODE -ne 0) {
        throw "FFmpeg failed while generating $Name"
    }
}

$sampleRates = @(44100, 48000, 96000, 192000)
$integerWavCodecs = @{
    16 = "pcm_s16le"
    24 = "pcm_s24le"
    32 = "pcm_s32le"
}

foreach ($sampleRate in $sampleRates) {
    foreach ($bitDepth in @(16, 24, 32)) {
        Invoke-Encoder "wav_${sampleRate}_${bitDepth}_stereo.wav" @(
            "-ar", $sampleRate,
            "-c:a", $integerWavCodecs[$bitDepth]
        )
    }

    Invoke-Encoder "wav_${sampleRate}_f32_stereo.wav" @(
        "-ar", $sampleRate,
        "-c:a", "pcm_f32le"
    )

    foreach ($bitDepth in @(16, 24)) {
        $sampleFormat = if ($bitDepth -eq 16) { "s16" } else { "s32" }
        Invoke-Encoder "flac_${sampleRate}_${bitDepth}_stereo.flac" @(
            "-ar", $sampleRate,
            "-sample_fmt", $sampleFormat,
            "-bits_per_raw_sample", $bitDepth,
            "-c:a", "flac"
        )
    }

    $flacOutput = Join-Path $output "flac_${sampleRate}_32_stereo.flac"
    & $flac --force --silent --no-padding --no-seektable --no-preserve-modtime --no-mid-side `
        "--output-name=$flacOutput" (Join-Path $output "wav_${sampleRate}_32_stereo.wav")
    if ($LASTEXITCODE -ne 0) {
        throw "FLAC failed while generating $flacOutput"
    }
}

foreach ($sampleRate in @(44100, 48000)) {
    Invoke-Encoder "mp3_${sampleRate}_cbr128_stereo.mp3" @(
        "-ar", $sampleRate,
        "-c:a", "libmp3lame",
        "-b:a", "128k"
    )
    Invoke-Encoder "mp3_${sampleRate}_cbr320_stereo.mp3" @(
        "-ar", $sampleRate,
        "-c:a", "libmp3lame",
        "-b:a", "320k"
    )
    Invoke-Encoder "mp3_${sampleRate}_vbr0_stereo.mp3" @(
        "-ar", $sampleRate,
        "-c:a", "libmp3lame",
        "-q:a", "0"
    )
}

& $ffmpeg @common "-ar" 44100 "-ac" 1 "-c:a" "pcm_s16le" (Join-Path $output "wav_44100_16_mono.wav")
if ($LASTEXITCODE -ne 0) { throw "FFmpeg failed while generating mono fixture" }

& $ffmpeg -hide_banner -loglevel error -y -fflags +bitexact -flags:a +bitexact `
    -f lavfi -i "sine=frequency=997:duration=4" -map_metadata -1 -ac 2 -ar 48000 `
    -sample_fmt s32 -bits_per_raw_sample 24 -c:a flac (Join-Path $output "seek_48000_24_stereo.flac")
if ($LASTEXITCODE -ne 0) { throw "FFmpeg failed while generating seek fixture" }

[System.IO.File]::WriteAllBytes((Join-Path $output "empty.wav"), [byte[]]@())
$validBytes = [System.IO.File]::ReadAllBytes((Join-Path $output "wav_44100_16_stereo.wav"))
[System.IO.File]::WriteAllBytes((Join-Path $output "truncated.wav"), $validBytes[0..31])
[System.IO.File]::WriteAllBytes((Join-Path $output "wav_content_as_flac.flac"), $validBytes)

$hashLines = Get-ChildItem -LiteralPath $output -File |
    Where-Object { $_.Extension -in @(".wav", ".flac", ".mp3") } |
    Sort-Object Name |
    ForEach-Object {
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
        "$hash  $($_.Name)"
    }
[System.IO.File]::WriteAllLines((Join-Path $output "SHA256SUMS.txt"), $hashLines)

Write-Host "Generated $($hashLines.Count) audio fixtures in $output"
