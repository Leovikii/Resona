$ErrorActionPreference = "Stop"

$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$iconRoot = (Resolve-Path -LiteralPath (Join-Path $projectRoot "src-tauri\icons")).Path
$sources = @(
    @{ Source = "assets\resona-file-mp3.svg"; Output = "file-mp3.ico" },
    @{ Source = "assets\resona-file-wav.svg"; Output = "file-wav.ico" },
    @{ Source = "assets\resona-file-flac.svg"; Output = "file-flac.ico" },
    @{ Source = "src-tauri\icons\taskbar-previous.svg"; Output = "taskbar-previous.ico" },
    @{ Source = "src-tauri\icons\taskbar-play.svg"; Output = "taskbar-play.ico" },
    @{ Source = "src-tauri\icons\taskbar-pause.svg"; Output = "taskbar-pause.ico" },
    @{ Source = "src-tauri\icons\taskbar-next.svg"; Output = "taskbar-next.ico" }
)

foreach ($entry in $sources) {
    $source = (Resolve-Path -LiteralPath (Join-Path $projectRoot $entry.Source)).Path
    $temporary = Join-Path $iconRoot ("generated-" + [System.IO.Path]::GetFileNameWithoutExtension($entry.Output))
    $resolvedTemporary = [System.IO.Path]::GetFullPath($temporary)
    if (-not $resolvedTemporary.StartsWith(
        $iconRoot + [System.IO.Path]::DirectorySeparatorChar,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Refusing to use an icon staging directory outside $iconRoot"
    }
    if (Test-Path -LiteralPath $resolvedTemporary) {
        Remove-Item -LiteralPath $resolvedTemporary -Recurse -Force
    }
    & npm run tauri -- icon $source --output $resolvedTemporary
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri icon generation failed for $source"
    }
    Copy-Item `
        -LiteralPath (Join-Path $resolvedTemporary "icon.ico") `
        -Destination (Join-Path $iconRoot $entry.Output) `
        -Force
    Remove-Item -LiteralPath $resolvedTemporary -Recurse -Force
}

Get-Item -LiteralPath ($sources | ForEach-Object { Join-Path $iconRoot $_.Output }) |
    Select-Object Name, Length
