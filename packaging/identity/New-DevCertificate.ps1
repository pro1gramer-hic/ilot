#Requires -Version 5.1
<#
.SYNOPSIS
    Crée le certificat de développement qui signe le package d'identité d'îlot.

.DESCRIPTION
    - Certificat auto-signé dont le sujet est le Publisher de AppxManifest.xml (CN=ilot-dev),
      rangé dans Cert:\CurrentUser\My avec une clé privée non exportable.
    - Sa partie publique (out\ilot-dev.cer) est ajoutée à Cert:\CurrentUser\TrustedPeople :
      Windows accepte alors, pour ton compte seulement, les packages signés avec.
    - Aucun fichier .pfx ni mot de passe : la clé privée ne quitte pas le magasin de Windows.
    - Pas besoin d'être administrateur. Relancer le script ne crée pas de doublon.
    Retrait : Unregister-Identity.ps1 -RemoveCertificate.

.EXAMPLE
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\identity\New-DevCertificate.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

[xml]$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot 'AppxManifest.xml')
$publisher = $manifest.Package.Identity.Publisher

$certificate = Get-ChildItem -Path Cert:\CurrentUser\My |
    Where-Object { $_.Subject -eq $publisher -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date).AddDays(7) } |
    Sort-Object -Property NotAfter -Descending |
    Select-Object -First 1

if ($certificate) {
    Write-Host "Certificat déjà présent : $publisher ($($certificate.Thumbprint))"
} else {
    # Paramètres de https://learn.microsoft.com/windows/msix/package/create-certificate-package-signing :
    # usage « signature de code » (1.3.6.1.5.5.7.3.3) et certificat final, pas une autorité.
    $certificate = New-SelfSignedCertificate -Type Custom `
        -Subject $publisher `
        -FriendlyName 'îlot (développement)' `
        -KeyUsage DigitalSignature `
        -KeyExportPolicy NonExportable `
        -NotAfter (Get-Date).AddYears(2) `
        -CertStoreLocation 'Cert:\CurrentUser\My' `
        -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
    Write-Host "Certificat créé : $publisher ($($certificate.Thumbprint))"
}

$out = Join-Path $PSScriptRoot 'out'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$publicPart = Join-Path $out 'ilot-dev.cer'
Export-Certificate -Cert $certificate -FilePath $publicPart | Out-Null

$trusted = Get-ChildItem -Path Cert:\CurrentUser\TrustedPeople | Where-Object { $_.Thumbprint -eq $certificate.Thumbprint }
if ($trusted) {
    Write-Host 'Déjà approuvé dans Cert:\CurrentUser\TrustedPeople.'
} else {
    Import-Certificate -FilePath $publicPart -CertStoreLocation 'Cert:\CurrentUser\TrustedPeople' | Out-Null
    Write-Host 'Partie publique ajoutée à Cert:\CurrentUser\TrustedPeople.'
}
Write-Host "OK. Étape suivante : Build-IdentityPackage.ps1"
