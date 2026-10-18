# Builds the portable zip and the Store package from a release build.
# Run from the repo root after `cargo build --release` and `engines/fetch.ps1`.
param(
	[string]$Out = "dist",
	# Also make a self-signed copy of the package for installing on a test PC.
	[switch]$TestSigned
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$version = ([regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version = "([^"]+)"')).Groups[1].Value
if (-not $version) { throw "no version in Cargo.toml" }
$exe = "target/release/tapwheel.exe"
$ffmpeg = "target/engines/ffmpeg.exe"
foreach ($f in @($exe, $ffmpeg)) { if (-not (Test-Path $f)) { throw "$f is missing" } }

Remove-Item -Recurse -Force $Out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$assets = Join-Path $Out "assets"
cargo run -q --release -p tapwheel-ui --example assets -- $assets
if ($LASTEXITCODE -ne 0) { throw "asset generation failed" }

function Copy-App($dest) {
	New-Item -ItemType Directory -Force -Path (Join-Path $dest "engines") | Out-Null
	Copy-Item $exe (Join-Path $dest "tapwheel.exe")
	Copy-Item $ffmpeg (Join-Path $dest "engines/ffmpeg.exe")
	Copy-Item LICENSE (Join-Path $dest "LICENSE.txt")
	Copy-Item THIRD-PARTY-NOTICES.md (Join-Path $dest "THIRD-PARTY-NOTICES.md")
}

# portable zip
$portable = Join-Path $Out "TapWheel"
Copy-App $portable
$zip = Join-Path $Out "TapWheel-$version-windows-x64.zip"
Compress-Archive -Path "$portable/*" -DestinationPath $zip
Write-Host "Portable: $zip"

# Store package
$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Directory |
	Where-Object { Test-Path (Join-Path $_.FullName "x64\makeappx.exe") } |
	Sort-Object { [version]$_.Name } | Select-Object -Last 1
if (-not $sdk) { throw "Windows SDK with makeappx.exe not found" }
$bin = Join-Path $sdk.FullName "x64"

$layout = Join-Path $Out "msix"
Copy-App $layout
New-Item -ItemType Directory -Force -Path (Join-Path $layout "Assets") | Out-Null
Get-ChildItem $assets -Filter "*.png" | Where-Object { $_.Name -ne "logo-512.png" } |
	Copy-Item -Destination (Join-Path $layout "Assets")
(Get-Content packaging/AppxManifest.xml -Raw).Replace('$VERSION$', $version) |
	Set-Content -Path (Join-Path $layout "AppxManifest.xml") -Encoding utf8

Push-Location $layout
& "$bin\makepri.exe" createconfig /cf priconfig.xml /dq en-US /o | Out-Null
& "$bin\makepri.exe" new /pr . /cf priconfig.xml /of resources.pri /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri failed" }
Remove-Item priconfig.xml
Pop-Location

$msix = Join-Path $Out "TapWheel-$version-x64.msix"
& "$bin\makeappx.exe" pack /d $layout /p $msix /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed" }
Write-Host "Store package: $msix"

if ($TestSigned) {
	# Same publisher name as the Store identity, so the package installs like the real one.
	$subject = "CN=50CBFD77-B27F-4D02-AB74-844DC5756318"
	$cert = New-SelfSignedCertificate -Type Custom -Subject $subject -KeyUsage DigitalSignature `
		-FriendlyName "TapWheel test" -CertStoreLocation "Cert:\CurrentUser\My" `
		-TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
	$test = Join-Path $Out "TapWheel-$version-x64-test.msix"
	Copy-Item $msix $test
	$pfx = Join-Path $Out "test.pfx"
	$pw = ConvertTo-SecureString -String "tapwheel-test" -Force -AsPlainText
	Export-PfxCertificate -Cert $cert -FilePath $pfx -Password $pw | Out-Null
	& "$bin\signtool.exe" sign /fd SHA256 /a /f $pfx /p "tapwheel-test" $test
	if ($LASTEXITCODE -ne 0) { throw "signtool failed" }
	Export-Certificate -Cert $cert -FilePath (Join-Path $Out "TapWheel-test.cer") | Out-Null
	Remove-Item $pfx
	Remove-Item $cert.PSPath
	Write-Host "Test package: $test"
}
