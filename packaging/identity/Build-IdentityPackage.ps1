#Requires -Version 5.1
<#
.SYNOPSIS
    Construit et signe le package d'identité d'îlot : out\ilot-identity.msix.

.DESCRIPTION
    1. Prépare out\stage : AppxManifest.xml et les logos (PNG tirés de src-tauri\icons\icon.ico).
    2. MakeAppx.exe pack /nv (le manifeste désigne ilot.exe, qui vit hors du package).
    3. SignTool.exe sign /fd SHA256 avec le certificat de New-DevCertificate.ps1.
    MakeAppx et SignTool viennent du Windows SDK (C:\Program Files (x86)\Windows Kits\10\bin).
    -NoSign construit seulement (pour vérifier le manifeste) : un package non signé ne
    s'enregistre pas.

.EXAMPLE
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\identity\Build-IdentityPackage.ps1
#>
[CmdletBinding()]
param(
    [switch]$NoSign
)

$ErrorActionPreference = 'Stop'

function Find-SdkTool([string]$Name) {
    $bin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $tool = Get-ChildItem -Path $bin -Directory -Filter '10.*' -ErrorAction SilentlyContinue |
        Sort-Object -Property { [version]$_.Name } -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
    if (-not $tool) {
        throw "$Name introuvable dans $bin : installe le Windows SDK (Visual Studio Installer, composant « Windows SDK »)."
    }
    $tool
}

function Save-Logo([System.Drawing.Icon]$Icon, [int]$Size, [string]$Path) {
    $source = (New-Object System.Drawing.Icon($Icon, 256, 256)).ToBitmap()
    $logo = New-Object System.Drawing.Bitmap($Size, $Size)
    $graphics = [System.Drawing.Graphics]::FromImage($logo)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.DrawImage($source, 0, 0, $Size, $Size)
    $graphics.Dispose()
    $logo.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $logo.Dispose()
    $source.Dispose()
}

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$out = Join-Path $PSScriptRoot 'out'
$stage = Join-Path $out 'stage'
$package = Join-Path $out 'ilot-identity.msix'

if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'Assets') | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'AppxManifest.xml') -Destination $stage

Add-Type -AssemblyName System.Drawing
$icon = New-Object System.Drawing.Icon((Join-Path $root 'src-tauri\icons\icon.ico'))
Save-Logo $icon 50 (Join-Path $stage 'Assets\StoreLogo.png')
Save-Logo $icon 150 (Join-Path $stage 'Assets\Square150x150Logo.png')
Save-Logo $icon 44 (Join-Path $stage 'Assets\Square44x44Logo.png')
$icon.Dispose()

$makeAppx = Find-SdkTool 'makeappx.exe'
& $makeAppx pack /o /d $stage /nv /p $package
if ($LASTEXITCODE -ne 0) { throw "MakeAppx a échoué (code $LASTEXITCODE) : voir les erreurs du manifeste ci-dessus." }

if ($NoSign) {
    Write-Host "Package construit sans signature : $package"
    return
}

[xml]$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot 'AppxManifest.xml')
$publisher = $manifest.Package.Identity.Publisher
$certificate = Get-ChildItem -Path Cert:\CurrentUser\My |
    Where-Object { $_.Subject -eq $publisher -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date) } |
    Sort-Object -Property NotAfter -Descending |
    Select-Object -First 1
if (-not $certificate) {
    throw "Aucun certificat « $publisher » : lance d'abord New-DevCertificate.ps1."
}

$signTool = Find-SdkTool 'signtool.exe'
& $signTool sign /fd SHA256 /sha1 $certificate.Thumbprint $package
if ($LASTEXITCODE -ne 0) { throw "SignTool a échoué (code $LASTEXITCODE)." }
Write-Host "Package signé : $package"
Write-Host 'Étape suivante : Register-Identity.ps1'
