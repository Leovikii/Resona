# SPDX-License-Identifier: GPL-3.0-only
# Run from a Windows MSVC developer shell, after npm run build:exe.
param([switch]$PrepareOnly, [string]$TestExecutable)

$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
$previousId = $env:RESONA_TEST_LYRICS_ID
$testId = 'io.github.vki.resona.lyrics-test-' + [Guid]::NewGuid().ToString('N')
$profile = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) $testId
$output = Join-Path $PWD 'src-tauri/target/windows-lyrics-test'
try {
    New-Item -ItemType Directory -Force $output | Out-Null
    if ($TestExecutable) {
        if ($PrepareOnly) { throw 'Choose either preparation or a prebuilt test executable' }
        $testExe = (Resolve-Path -LiteralPath $TestExecutable).Path
    } else {
        $artifacts = cargo test --manifest-path src-tauri/Cargo.toml --locked --features tauri/custom-protocol --lib --no-run --message-format=json
        if ($LASTEXITCODE -ne 0) { throw 'Native lyrics test build failed' }
        $compiled = @($artifacts | ForEach-Object { ConvertFrom-Json $_ } |
            Where-Object { $_.reason -eq 'compiler-artifact' -and $_.profile.test -and $_.executable } |
            Select-Object -ExpandProperty executable)
        if ($compiled.Count -ne 1) { throw 'Expected exactly one library test executable' }
        $testExe = Join-Path $output 'resona-native-tests.exe'
        Copy-Item -LiteralPath $compiled[0] -Destination $testExe -Force

        # Cargo library tests omit Tauri's application resource, including Common Controls v6.
        $manifestTool = (Get-Command mt.exe -ErrorAction SilentlyContinue).Source
        if (-not $manifestTool) {
            $sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
            $manifestTool = Get-ChildItem -LiteralPath $sdkBin -Directory |
                Where-Object Name -Match '^\d+\.\d+\.\d+\.\d+$' |
                Sort-Object { [version]$_.Name } -Descending |
                ForEach-Object { Join-Path $_.FullName 'x64/mt.exe' } |
                Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        }
        if (-not $manifestTool) { throw 'Windows SDK manifest tool was not found' }
        $manifest = Join-Path $output 'app.manifest'
        & $manifestTool '-inputresource:src-tauri/target/release/resona.exe;#1' "-out:$manifest"
        if ($LASTEXITCODE -ne 0) { throw 'Cannot extract the Tauri application manifest' }
        & $manifestTool -manifest $manifest "-outputresource:$testExe;#1"
        if ($LASTEXITCODE -ne 0) { throw 'Cannot embed the native test manifest' }
        if ($PrepareOnly) { return }
    }

    $env:RESONA_TEST_LYRICS_ID = $testId
    $stdout = Join-Path $output 'stdout.log'
    $stderr = Join-Path $output 'stderr.log'
    $testProcess = Start-Process -FilePath $testExe -ArgumentList @(
        'native_lyrics_geometry_and_lock_lifecycle', '--ignored', '--nocapture', '--test-threads=1'
    ) -RedirectStandardOutput $stdout -RedirectStandardError $stderr -WindowStyle Hidden -PassThru
    if (-not $testProcess.WaitForExit(30000)) {
        $testProcess.Kill()
        $testProcess.WaitForExit()
        throw 'Native lyrics test exceeded 30 seconds'
    }
    Get-Content $stdout
    Get-Content $stderr
    if ($testProcess.ExitCode -ne 0) { throw "Native lyrics test failed: $($testProcess.ExitCode)" }
    if (-not (Select-String -Path $stdout -Pattern 'native_lyrics_geometry_and_lock_lifecycle \.\.\. ok' -Quiet)) {
        throw 'The native lyrics regression did not run successfully'
    }
    if (Select-String -Path $stderr -Pattern 'topmost observer removal.*failed' -Quiet) {
        throw 'Native WinEvent hooks were not released successfully'
    }
} finally {
    $env:RESONA_TEST_LYRICS_ID = $previousId
    # WebView2 releases its profile only after the test process exits.
    # This exact, generated application directory belongs exclusively to this run.
    $expectedParent = [IO.Path]::GetFullPath([Environment]::GetFolderPath('LocalApplicationData'))
    if ((Split-Path ([IO.Path]::GetFullPath($profile)) -Parent) -ne $expectedParent -or
        (Split-Path $profile -Leaf) -ne $testId) { throw 'Unexpected test profile path' }
    if (Test-Path -LiteralPath $profile) { Remove-Item -LiteralPath $profile -Recurse -Force }
    Pop-Location
}
