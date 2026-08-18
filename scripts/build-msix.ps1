param(
    [switch]$SkipBuild,
    [string]$CertSubject = 'CN=SimpleFox, O=SimpleFox, C=GB'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

$version = (Get-Content (Join-Path $root 'version.json') -Raw | ConvertFrom-Json).version
$release = Join-Path $root 'src-tauri\target\release'
$stage = Join-Path $root 'src-tauri\target\msix-stage'
$outDir = Join-Path $root 'src-tauri\target\release\bundle\msix'
$msix = Join-Path $outDir "Nuru_${version}_x64.msix"

Write-Host "Nuru $version -> MSIX" -ForegroundColor Cyan

function Find-SdkTool([string]$name) {
    $bases = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "$env:ProgramFiles\Windows Kits\10\bin"
    )
    $found = foreach ($b in $bases) {
        if (Test-Path $b) {
            Get-ChildItem $b -Directory -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending |
                ForEach-Object { Join-Path $_.FullName "x64\$name" } |
                Where-Object { Test-Path $_ }
        }
    }
    $tool = $found | Select-Object -First 1
    if (-not $tool) { throw "$name not found. Install the Windows 10/11 SDK." }
    $tool
}

$makeappx = Find-SdkTool 'makeappx.exe'
$signtool = Find-SdkTool 'signtool.exe'
$makepri = Find-SdkTool 'makepri.exe'

if (-not $SkipBuild) {
    Push-Location $root
    try {
        & npm run app:build
        if ($LASTEXITCODE -ne 0) { throw 'app:build failed' }
    } finally { Pop-Location }
}

if (-not (Test-Path (Join-Path $release 'nuru.exe'))) {
    throw "nuru.exe not found in $release. Build first, or drop -SkipBuild."
}

if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Path $stage -Force | Out-Null

Copy-Item (Join-Path $release 'nuru.exe') $stage
foreach ($dll in Get-ChildItem $release -Filter *.dll -ErrorAction SilentlyContinue) {
    Copy-Item $dll.FullName $stage
}
Copy-Item (Join-Path $root 'resources\packs') (Join-Path $stage 'packs') -Recurse
Copy-Item (Join-Path $root 'msix\Assets') (Join-Path $stage 'Assets') -Recurse

$manifest = Get-Content (Join-Path $root 'msix\AppxManifest.template.xml') -Raw
$manifest = $manifest.
    Replace('{{IDENTITY_NAME}}', 'SimpleFox.Nuru').
    Replace('{{PUBLISHER}}', $CertSubject).
    Replace('{{PUBLISHER_DISPLAY}}', 'SimpleFox').
    Replace('{{DISPLAY_NAME}}', 'Nuru').
    Replace('{{DESCRIPTION}}', 'Play any combination of ambient sounds, seamlessly looped.').
    Replace('{{VERSION}}', $version)
Set-Content (Join-Path $stage 'AppxManifest.xml') $manifest -Encoding UTF8

$priConfig = Join-Path $stage 'priconfig.xml'
& $makepri createconfig /cf $priConfig /dq en-US /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'makepri createconfig failed' }

Push-Location $stage
try {
    & $makepri new /pr $stage /cf $priConfig /of (Join-Path $stage 'resources.pri') /mn (Join-Path $stage 'AppxManifest.xml') /o | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'makepri new failed' }
} finally { Pop-Location }
Remove-Item $priConfig -Force -ErrorAction SilentlyContinue

New-Item -ItemType Directory -Path $outDir -Force | Out-Null
if (Test-Path $msix) { Remove-Item $msix -Force }

& $makeappx pack /o /d $stage /p $msix
if ($LASTEXITCODE -ne 0) { throw 'makeappx failed' }

$cert = Get-ChildItem Cert:\CurrentUser\My |
    Where-Object { $_.Subject -eq $CertSubject -and $_.NotAfter -gt (Get-Date) } |
    Select-Object -First 1

if (-not $cert) {
    Write-Host "Creating a self-signed certificate for $CertSubject" -ForegroundColor Yellow
    $cert = New-SelfSignedCertificate -Type Custom -Subject $CertSubject `
        -KeyUsage DigitalSignature -FriendlyName 'Nuru development signing' `
        -CertStoreLocation 'Cert:\CurrentUser\My' `
        -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}') `
        -NotAfter (Get-Date).AddYears(3)
}

& $signtool sign /fd SHA256 /a /sha1 $cert.Thumbprint $msix
if ($LASTEXITCODE -ne 0) { throw 'signtool failed' }

$cerPath = Join-Path $outDir 'Nuru-dev-cert.cer'
Export-Certificate -Cert $cert -FilePath $cerPath | Out-Null

$size = [math]::Round((Get-Item $msix).Length / 1MB, 1)
Write-Host ''
Write-Host "MSIX: $msix ($size MB)" -ForegroundColor Green
Write-Host "Signed with $($cert.Thumbprint)"
Write-Host ''
Write-Host 'Windows will not install this until the certificate is trusted.' -ForegroundColor Yellow
Write-Host 'Run this once, as administrator:'
Write-Host "  Import-Certificate -FilePath `"$cerPath`" -CertStoreLocation Cert:\LocalMachine\TrustedPeople" -ForegroundColor Cyan
Write-Host ''
Write-Host 'Then double-click the .msix to install through App Installer.'
