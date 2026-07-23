param(
    [string]$IconPath = (Join-Path $PSScriptRoot "..\src-tauri\icons\icon.png"),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot "..\src-tauri\windows\nsis")
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$resolvedIcon = (Resolve-Path -LiteralPath $IconPath).Path
$resolvedOutput = [System.IO.Path]::GetFullPath($OutputDirectory)
[System.IO.Directory]::CreateDirectory($resolvedOutput) | Out-Null

function New-InstallerBitmap {
    param(
        [int]$Width,
        [int]$Height,
        [string]$OutputPath,
        [scriptblock]$Draw
    )

    $bitmap = [System.Drawing.Bitmap]::new(
        $Width,
        $Height,
        [System.Drawing.Imaging.PixelFormat]::Format24bppRgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    try {
        & $Draw $graphics $Width $Height
        $bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

$icon = [System.Drawing.Image]::FromFile($resolvedIcon)
try {
    New-InstallerBitmap -Width 150 -Height 57 -OutputPath (Join-Path $resolvedOutput "header.bmp") -Draw {
        param($graphics, $width, $height)
        $bounds = [System.Drawing.Rectangle]::new(0, 0, $width, $height)
        $gradient = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            $bounds,
            [System.Drawing.Color]::FromArgb(15, 13, 27),
            [System.Drawing.Color]::FromArgb(84, 55, 214),
            12
        )
        $titleFont = [System.Drawing.Font]::new("Segoe UI Semibold", 14, [System.Drawing.FontStyle]::Bold)
        $detailFont = [System.Drawing.Font]::new("Segoe UI", 7.5)
        $white = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
        $muted = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(218, 224, 255))
        try {
            $graphics.FillRectangle($gradient, $bounds)
            $graphics.DrawImage($icon, 10, 7, 42, 42)
            $graphics.DrawString("Resona", $titleFont, $white, 59, 7)
            $graphics.DrawString("Local audio. Yours.", $detailFont, $muted, 60, 31)
        }
        finally {
            $gradient.Dispose()
            $titleFont.Dispose()
            $detailFont.Dispose()
            $white.Dispose()
            $muted.Dispose()
        }
    }

    New-InstallerBitmap -Width 164 -Height 314 -OutputPath (Join-Path $resolvedOutput "sidebar.bmp") -Draw {
        param($graphics, $width, $height)
        $bounds = [System.Drawing.Rectangle]::new(0, 0, $width, $height)
        $gradient = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            $bounds,
            [System.Drawing.Color]::FromArgb(13, 12, 25),
            [System.Drawing.Color]::FromArgb(97, 60, 224),
            72
        )
        $wavePen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(46, 229, 220, 255), 1.3)
        $titleFont = [System.Drawing.Font]::new("Segoe UI Semibold", 21, [System.Drawing.FontStyle]::Bold)
        $detailFont = [System.Drawing.Font]::new("Segoe UI", 9)
        $white = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
        $muted = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(225, 230, 255))
        $center = [System.Drawing.StringFormat]::new()
        $center.Alignment = [System.Drawing.StringAlignment]::Center
        try {
            $graphics.FillRectangle($gradient, $bounds)
            for ($index = 0; $index -lt 5; $index++) {
                $diameter = 105 + ($index * 24)
                $graphics.DrawEllipse($wavePen, 82 - ($diameter / 2), 240 - ($diameter / 2), $diameter, $diameter)
            }
            $graphics.DrawImage($icon, 51, 44, 62, 62)
            $graphics.DrawString(
                "Resona",
                $titleFont,
                $white,
                [System.Drawing.RectangleF]::new(0, 119, $width, 42),
                $center
            )
            $graphics.DrawString(
                "Your music stays local.`nYour listening stays yours.",
                $detailFont,
                $muted,
                [System.Drawing.RectangleF]::new(14, 168, $width - 28, 48),
                $center
            )
        }
        finally {
            $gradient.Dispose()
            $wavePen.Dispose()
            $titleFont.Dispose()
            $detailFont.Dispose()
            $white.Dispose()
            $muted.Dispose()
            $center.Dispose()
        }
    }
}
finally {
    $icon.Dispose()
}

Write-Output "Generated NSIS artwork in $resolvedOutput"
