# Downloads the pinned converters into target/engines and checks their hashes.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "target/engines"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$toml = Get-Content (Join-Path $PSScriptRoot "engines.toml") -Raw
$value = { param($name) ([regex]::Match($toml, "(?m)^$name\s*=\s*`"([^`"]+)`"")).Groups[1].Value }
$url = & $value "url"
$sha = & $value "sha256"
$file = & $value "file"

$exe = Join-Path $out "ffmpeg.exe"
$stamp = Join-Path $out "ffmpeg.sha256"
if ((Test-Path $exe) -and (Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq $sha)) {
	Write-Host "ffmpeg already fetched"
	exit 0
}

$zip = Join-Path $out "ffmpeg.zip"
Write-Host "Downloading $url"
$ProgressPreference = "SilentlyContinue"
Invoke-WebRequest -Uri $url -OutFile $zip
$got = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
if ($got -ne $sha) {
	Remove-Item $zip
	throw "ffmpeg hash mismatch: expected $sha, got $got"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($zip)
try {
	$entry = $archive.Entries | Where-Object { $_.FullName -eq $file }
	if (-not $entry) { throw "$file is not in the archive" }
	[System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $exe, $true)
} finally {
	$archive.Dispose()
}
Remove-Item $zip
Set-Content -Path $stamp -Value $sha -NoNewline
Write-Host "ffmpeg ready at $exe"
