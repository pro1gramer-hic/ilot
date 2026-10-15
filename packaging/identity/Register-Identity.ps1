#Requires -Version 5.1
<#
.SYNOPSIS
    Enregistre le package d'identité d'îlot pour ton compte, rattaché au dossier de ilot.exe.

.DESCRIPTION
    Add-AppxPackage -Path out\ilot-identity.msix -ExternalLocation <dossier de ilot.exe>.
    Par défaut, le dossier est target\debug du dépôt (celui de « npm.cmd run tauri dev »).
    Un ancien enregistrement est d'abord retiré. Pas besoin d'être administrateur.
    Ensuite, relance îlot : l'identité s'applique au démarrage du programme.

.PARAMETER ExternalLocation
    Dossier qui contient ilot.exe (par exemple le dossier d'installation d'îlot).

.EXAMPLE
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\identity\Register-Identity.ps1
#>
[CmdletBinding()]
param(
    [string]$ExternalLocation = (Join-Path $PSScriptRoot '..\..\target\debug')
)

$ErrorActionPreference = 'Stop'

$location = (Resolve-Path -LiteralPath $ExternalLocation).Path
if (-not (Test-Path -LiteralPath (Join-Path $location 'ilot.exe'))) {
    throw "ilot.exe introuvable dans $location : lance d'abord « npm.cmd run tauri dev » (ou « cargo build -p ilot »)."
}
$package = Join-Path $PSScriptRoot 'out\ilot-identity.msix'
if (-not (Test-Path -LiteralPath $package)) {
    throw "Package introuvable : lance d'abord Build-IdentityPackage.ps1."
}

[xml]$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot 'AppxManifest.xml')
$name = $manifest.Package.Identity.Name

$previous = Get-AppxPackage -Name $name
if ($previous) {
    Write-Host "Ancien enregistrement retiré : $($previous.PackageFullName)"
    $previous | Remove-AppxPackage
}

Add-AppxPackage -Path $package -ExternalLocation $location

$registered = Get-AppxPackage -Name $name
if (-not $registered) { throw "Windows n'a pas gardé le package $name." }
Write-Host "Identité enregistrée : $($registered.PackageFullName)"
Write-Host "Dossier rattaché     : $location"
Write-Host 'Relance îlot pour qu''il prenne son identité.'
