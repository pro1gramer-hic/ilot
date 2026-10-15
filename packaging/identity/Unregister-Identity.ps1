#Requires -Version 5.1
<#
.SYNOPSIS
    Retire le package d'identité d'îlot (et, sur demande, le certificat de développement).

.DESCRIPTION
    Get-AppxPackage <nom> | Remove-AppxPackage. îlot redevient une app sans identité :
    il continue de marcher, sans les notifications des messageries.
    -RemoveCertificate retire aussi le certificat de Cert:\CurrentUser\My et de
    Cert:\CurrentUser\TrustedPeople, et supprime out\ilot-dev.cer.

.EXAMPLE
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\identity\Unregister-Identity.ps1 -RemoveCertificate
#>
[CmdletBinding()]
param(
    [switch]$RemoveCertificate
)

$ErrorActionPreference = 'Stop'

[xml]$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot 'AppxManifest.xml')
$name = $manifest.Package.Identity.Name
$publisher = $manifest.Package.Identity.Publisher

$registered = Get-AppxPackage -Name $name
if ($registered) {
    $registered | Remove-AppxPackage
    Write-Host "Package retiré : $($registered.PackageFullName)"
} else {
    Write-Host "Aucun package $name enregistré."
}

if ($RemoveCertificate) {
    foreach ($store in 'Cert:\CurrentUser\TrustedPeople', 'Cert:\CurrentUser\My') {
        Get-ChildItem -Path $store | Where-Object { $_.Subject -eq $publisher } | ForEach-Object {
            Remove-Item -LiteralPath $_.PSPath
            Write-Host "Certificat retiré de $store : $($_.Thumbprint)"
        }
    }
    $publicPart = Join-Path $PSScriptRoot 'out\ilot-dev.cer'
    if (Test-Path -LiteralPath $publicPart) { Remove-Item -LiteralPath $publicPart }
}
