param(
    [string]$Exe = "D:\GitHub Repos\Nuru-Audio-Player\src-tauri\target\release\nuru.exe",
    [int]$Width = 1240,
    [int]$Height = 800,
    [int]$Settle = 15
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'docs\shots'
New-Item -ItemType Directory -Force -Path $out | Out-Null

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class Cap {
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
'@

Get-Process -Name nuru -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

$proc = Start-Process $Exe -PassThru
Start-Sleep -Seconds $Settle
$proc.Refresh()

$handle = $proc.MainWindowHandle
if ($handle -eq [IntPtr]::Zero) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    throw 'Nuru window not found'
}

[void][Cap]::SetWindowPos($handle, [IntPtr]::Zero, 100, 80, $Width, $Height, 0x0040)
Start-Sleep -Seconds 3

function Capture([string]$name) {
    $r = New-Object Cap+RECT
    [void][Cap]::GetClientRect($handle, [ref]$r)
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { throw 'window has no client area' }

    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    $ok = [Cap]::PrintWindow($handle, $hdc, 2)
    $g.ReleaseHdc($hdc)
    $g.Dispose()

    if (-not $ok) { $bmp.Dispose(); throw 'PrintWindow failed' }

    $pixels = 0
    for ($x = 0; $x -lt $w; $x += 40) {
        for ($y = 0; $y -lt $h; $y += 40) {
            if ($bmp.GetPixel($x, $y).GetBrightness() -gt 0.04) { $pixels++ }
        }
    }
    if ($pixels -lt 10) {
        $bmp.Dispose()
        throw 'window rendered blank, the WebView surface is not capturable this way'
    }

    $path = Join-Path $out "$name.png"
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    "  $name.png  ${w}x${h}"
}

try {
    Capture 'mixer'
} finally {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}
