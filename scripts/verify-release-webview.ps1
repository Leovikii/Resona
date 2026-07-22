param(
    [string]$Executable = (Join-Path $PSScriptRoot "..\src-tauri\target\release\resona.exe"),
    [int]$TimeoutSeconds = 20,
    [int]$SettlingMilliseconds = 1000,
    [string]$CapturePath
)

$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class ResonaWindowProbe {
    private delegate bool EnumWindowsCallback(IntPtr hWnd, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential)]
    public struct Rect {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetWindowRect(IntPtr hWnd, out Rect rect);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetWindowPos(
        IntPtr hWnd,
        IntPtr hWndInsertAfter,
        int x,
        int y,
        int width,
        int height,
        uint flags
    );

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(EnumWindowsCallback callback, IntPtr lParam);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowTextLength(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr dpiContext);

    public static IntPtr FindLargestVisibleWindow(int processId) {
        IntPtr largestWindow = IntPtr.Zero;
        long largestArea = 0;
        EnumWindows((hWnd, _) => {
            uint ownerProcessId;
            GetWindowThreadProcessId(hWnd, out ownerProcessId);
            Rect rect;
            if (ownerProcessId != processId
                || !IsWindowVisible(hWnd)
                || GetWindowTextLength(hWnd) == 0
                || !GetWindowRect(hWnd, out rect)) {
                return true;
            }

            long width = Math.Max(0, rect.Right - rect.Left);
            long height = Math.Max(0, rect.Bottom - rect.Top);
            long area = width * height;
            if (area > largestArea) {
                largestArea = area;
                largestWindow = hWnd;
            }
            return true;
        }, IntPtr.Zero);
        return largestWindow;
    }
}
"@

function Test-MainWindowReady {
    param([IntPtr]$Handle)

    $null = [ResonaWindowProbe]::SetThreadDpiAwarenessContext((New-Object IntPtr(-4)))
    if ($Handle -eq [IntPtr]::Zero -or -not [ResonaWindowProbe]::IsWindowVisible($Handle)) {
        return $false
    }

    $rect = New-Object ResonaWindowProbe+Rect
    if (-not [ResonaWindowProbe]::GetWindowRect($Handle, [ref]$rect)) {
        return $false
    }

    return ($rect.Right - $rect.Left) -ge 320 -and ($rect.Bottom - $rect.Top) -ge 320
}

function Save-WindowCapture {
    param(
        [IntPtr]$Handle,
        [string]$Path
    )

    $null = [ResonaWindowProbe]::SetThreadDpiAwarenessContext((New-Object IntPtr(-4)))
    $topmost = New-Object IntPtr(-1)
    $noMoveNoSizeShow = 0x0001 -bor 0x0002 -bor 0x0040
    if (-not [ResonaWindowProbe]::SetWindowPos($Handle, $topmost, 0, 0, 0, 0, $noMoveNoSizeShow)) {
        throw "Could not bring the Resona window forward for capture."
    }
    $null = [ResonaWindowProbe]::SetForegroundWindow($Handle)
    Start-Sleep -Milliseconds 250

    $rect = New-Object ResonaWindowProbe+Rect
    if (-not [ResonaWindowProbe]::GetWindowRect($Handle, [ref]$rect)) {
        throw "Could not read the Resona window bounds."
    }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) {
        throw "Resona reported invalid window bounds ($width x $height)."
    }

    Add-Type -AssemblyName System.Drawing
    $resolvedCapturePath = [System.IO.Path]::GetFullPath($Path)
    $captureDirectory = [System.IO.Path]::GetDirectoryName($resolvedCapturePath)
    if (-not [string]::IsNullOrWhiteSpace($captureDirectory)) {
        $null = New-Item -ItemType Directory -Path $captureDirectory -Force
    }

    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
        $bitmap.Save($resolvedCapturePath, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }

    Write-Output "Release main window captured at: $resolvedCapturePath"
}

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
$windowHandle = [IntPtr]::Zero
try {
    $process = Start-Process -FilePath $resolvedExecutable -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)

    while ([DateTime]::UtcNow -lt $deadline) {
        if ($process.HasExited) {
            throw "Resona exited before its main window became ready."
        }

        $windowHandle = [ResonaWindowProbe]::FindLargestVisibleWindow($process.Id)
        if (Test-MainWindowReady -Handle $windowHandle) {
            break
        }

        Start-Sleep -Milliseconds 200
    }

    if (-not (Test-MainWindowReady -Handle $windowHandle)) {
        throw "Timed out waiting for the visible Resona main window to reach a usable size."
    }
    Start-Sleep -Milliseconds $SettlingMilliseconds
    $windowHandle = [ResonaWindowProbe]::FindLargestVisibleWindow($process.Id)
    if ($process.HasExited -or -not (Test-MainWindowReady -Handle $windowHandle)) {
        throw "Resona did not remain ready after its UI settling interval."
    }
    Write-Output "Release main window verified (HWND $windowHandle)"
    if (-not [string]::IsNullOrWhiteSpace($CapturePath)) {
        Save-WindowCapture -Handle $windowHandle -Path $CapturePath
    }
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        $null = $process.CloseMainWindow()
        if (-not $process.WaitForExit(3000)) {
            Stop-Process -Id $process.Id -Force
        }
    }
}
