param(
    [string] $Platform,
	[string] $Configuration,
	[string] $TrustedSigningMetadata,
	[string] $DigestAlgorithm,
	[string] $TimestampServer
)

$ErrorActionPreference = "Stop"

$msixPackageFolder = (Get-ChildItem "$PSScriptRoot\..\..\AppPackage\AppPackages").FullName
$msixPackage = (Get-ChildItem "$msixPackageFolder" -Filter "*.msix").FullName

[xml]$config = Get-Content "$PSScriptRoot\..\..\TranslucentTB\packages.config"

$trustedSigningClientVersion = ($config.packages.package | Where { $_.id -eq "Microsoft.Trusted.Signing.Client" }).version
$sdkVersion = ($config.packages.package | Where { $_.id -eq "Microsoft.Windows.SDK.BuildTools" }).version

$trustedSigningClientFolder = "$PSScriptRoot\..\..\packages\Microsoft.Trusted.Signing.Client.$trustedSigningClientVersion\bin"
$sdkFolder = (Get-ChildItem "$PSScriptRoot\..\..\packages\Microsoft.Windows.SDK.BuildTools.$sdkVersion\bin").FullName

&"$sdkFolder\x64\signtool.exe" sign /ph /tr "$TimestampServer" /td $DigestAlgorithm /fd $DigestAlgorithm /dlib "$trustedSigningClientFolder\x64\Azure.CodeSigning.Dlib.dll" /dmdf "$TrustedSigningMetadata" "$msixPackage"
