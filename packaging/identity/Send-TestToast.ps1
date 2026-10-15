#Requires -Version 5.1
<#
.SYNOPSIS
    Affiche une notification Windows factice, pour tester la lecture sans message privé.

.DESCRIPTION
    La notification vient de « Windows PowerShell » (aucune règle d'îlot ne la reconnaît) :
    la sonde « ilot.exe --notifications-probe » doit la lister avec ses textes.
    Elle imite WhatsApp Web dans Chrome : titre, message, puis la ligne du site, que Windows
    ne transmet pas aux programmes qui lisent les notifications.

.EXAMPLE
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\identity\Send-TestToast.ps1
#>
[CmdletBinding()]
param(
    [string]$Title = 'Test îlot',
    [string]$Message = 'Message factice pour la sonde des notifications.'
)

$ErrorActionPreference = 'Stop'

$null = [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime]
$null = [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime]

$escape = [System.Security.SecurityElement]
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml(@"
<toast>
  <visual>
    <binding template="ToastGeneric">
      <text>$($escape::Escape($Title))</text>
      <text>$($escape::Escape($Message))</text>
      <text placement="attribution">web.whatsapp.com</text>
    </binding>
  </visual>
</toast>
"@)

# Identifiant d'application de Windows PowerShell : il a déjà le droit d'afficher des notifications.
$powershell = '{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe'
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($powershell).Show(
    [Windows.UI.Notifications.ToastNotification]::new($xml))
Write-Host 'Notification factice envoyée (application : Windows PowerShell).'
