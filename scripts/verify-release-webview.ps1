param(
    [string]$Executable = (Join-Path $PSScriptRoot "..\src-tauri\target\release\resona.exe"),
    [int]$TimeoutSeconds = 20
)

$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class ResonaWindowProbe {
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindowVisible(IntPtr hWnd);
}
"@

$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$existingProcesses = @(Get-Process -Name "resona" -ErrorAction SilentlyContinue)
if ($existingProcesses.Count -gt 0) {
    throw "Close the running Resona instance before verifying the release WebView."
}

$devServer = Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue
if ($null -ne $devServer) {
    throw "Stop the Vite development server on port 1420 before verifying the release WebView."
}

$process = $null
try {
    $process = Start-Process -FilePath $resolvedExecutable -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)

    while ([DateTime]::UtcNow -lt $deadline) {
        if ($process.HasExited) {
            throw "Resona exited before its main window became ready."
        }

        if ($process.MainWindowHandle -ne 0 -and [ResonaWindowProbe]::IsWindowVisible($process.MainWindowHandle)) {
            $process.Refresh()
            break
        }

        Start-Sleep -Milliseconds 200
    }

    $process.Refresh()
    if ($process.MainWindowHandle -eq 0 -or -not [ResonaWindowProbe]::IsWindowVisible($process.MainWindowHandle)) {
        throw "Timed out waiting for the visible Resona main window."
    }
    Write-Output "Release main window verified (HWND $($process.MainWindowHandle))"
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        $null = $process.CloseMainWindow()
        if (-not $process.WaitForExit(3000)) {
            Stop-Process -Id $process.Id -Force
        }
    }
}
