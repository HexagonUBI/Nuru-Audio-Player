param([string]$MsiPath)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

if (-not $MsiPath) {
    $dir = Join-Path $root 'src-tauri\target\release\bundle\msi'
    if (-not (Test-Path $dir)) { Write-Host 'no msi bundle directory, nothing to patch'; exit 0 }
    $found = Get-ChildItem $dir -Filter *.msi -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $found) { Write-Host 'no msi found, nothing to patch'; exit 0 }
    $MsiPath = $found.FullName
}

$msiOpenDatabaseModeTransact = 1
$installer = New-Object -ComObject WindowsInstaller.Installer
$db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($MsiPath, $msiOpenDatabaseModeTransact))

function Invoke-Msi([string]$sql) {
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @($sql))
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null)
    $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null)
}

function Get-MsiProperty([string]$name) {
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @("SELECT Value FROM Property WHERE Property='$name'"))
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null)
    $rec = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
    $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null)
    if ($null -eq $rec) { return $null }
    $rec.GetType().InvokeMember('StringData', 'GetProperty', $null, $rec, 1)
}

$changes = @()

if (Get-MsiProperty 'ARPNOREPAIR') {
    Invoke-Msi "DELETE FROM Property WHERE Property='ARPNOREPAIR'"
    $changes += 'enabled Repair in Programs and Features'
}

if (Get-MsiProperty 'ARPNOMODIFY') {
    Invoke-Msi "DELETE FROM Property WHERE Property='ARPNOMODIFY'"
    $changes += 'enabled Change, which runs the installer again over an existing install'
}

$db.GetType().InvokeMember('Commit', 'InvokeMethod', $null, $db, $null)
[void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($db)
[void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($installer)
[GC]::Collect()

if ($changes.Count -eq 0) {
    Write-Host "msi already patched: $(Split-Path $MsiPath -Leaf)"
} else {
    Write-Host "patched $(Split-Path $MsiPath -Leaf)"
    foreach ($c in $changes) { Write-Host "  $c" }
}
