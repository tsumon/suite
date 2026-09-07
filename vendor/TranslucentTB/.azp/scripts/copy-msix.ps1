param(
	[string] $OutputDir
)

$ErrorActionPreference = "Stop"

New-Item -Path "$OutputDir" -ItemType "Directory" -Force

$childItem = Get-ChildItem "$PSScriptRoot\..\..\AppPackage\AppPackages"

if ($childItem.PSIsContainer)
{
	$msixPackage = (Get-ChildItem $childItem.FullName -Filter "*.msix").FullName
}
else
{
	$msixPackage = $childItem.FullName
}

Copy-Item -Path "$msixPackage" -Destination "$OutputDir"
