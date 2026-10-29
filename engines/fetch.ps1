# Downloads the pinned engines into target/engines and checks their hashes.
# -Arch arm64 fetches the Windows on Arm builds of FFmpeg and ONNX Runtime instead.
param([ValidateSet("x64", "arm64")][string]$Arch = "x64")
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "target/engines"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$toml = Get-Content (Join-Path $PSScriptRoot "engines.toml") -Raw
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Section($name) {
	$body = ([regex]::Match($toml, "(?ms)^\[$name\]\s*(.*?)(?=^\[|\z)")).Groups[1].Value
	$get = { param($key) ([regex]::Match($body, "(?m)^$key\s*=\s*`"([^`"]*)`"")).Groups[1].Value }
	$s = @{ url = & $get "url"; sha = & $get "sha256"; sums = & $get "sums"; files = (& $get "files") -split "\s+" | Where-Object { $_ } }
	if (-not $s.sha -and $s.sums) {
		# a build that's replaced over time: its hash comes from the maker's published list
		$file = Split-Path $s.url -Leaf
		# saved to a file and read as text: the list is served as bytes
		$sumsFile = Join-Path ([System.IO.Path]::GetTempPath()) "tapwheel-sums.txt"
		Invoke-WebRequest -Uri $s.sums -UseBasicParsing -OutFile $sumsFile
		$list = Get-Content $sumsFile
		Remove-Item $sumsFile
		$line = $list | Where-Object { ($_ -split "\s+")[1] -eq $file } | Select-Object -First 1
		if (-not $line) { throw "$file is not in $($s.sums)" }
		$s.sha = ($line -split "\s+")[0].ToLower()
	}
	if (-not $s.sha) { throw "no hash for [$name]" }
	$s
}

function Fetch($name, $s) {
	$stamp = Join-Path $out "$name.sha256"
	if ((Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq $s.sha)) {
		Write-Host "$name already fetched"
		return $false
	}
	$download = Join-Path $out "$name.download"
	Write-Host "Downloading $($s.url)"
	Invoke-WebRequest -Uri $s.url -OutFile $download
	$got = (Get-FileHash -Algorithm SHA256 $download).Hash.ToLower()
	if ($got -ne $s.sha) {
		Remove-Item $download
		throw "$name hash mismatch: expected $($s.sha), got $got"
	}
	if ($s.files.Count -gt 0) {
		$archive = [System.IO.Compression.ZipFile]::OpenRead($download)
		try {
			foreach ($file in $s.files) {
				$entry = $archive.Entries | Where-Object { $_.FullName -eq $file }
				if (-not $entry) { throw "$file is not in the $name archive" }
				[System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $out (Split-Path $file -Leaf)), $true)
			}
		} finally {
			$archive.Dispose()
		}
		Remove-Item $download
	}
	Set-Content -Path $stamp -Value $s.sha -NoNewline
	return $true
}

$suffix = if ($Arch -eq "arm64") { "-arm64" } else { "" }
Fetch "ffmpeg" (Section "ffmpeg$suffix") | Out-Null
Fetch "onnxruntime" (Section "onnxruntime$suffix") | Out-Null

$model = Section "model"
$fresh = Fetch "model" $model
if ($fresh -or -not (Test-Path (Join-Path $out "background.f16"))) {
	python -m pip install --quiet "onnx==1.18.0" "numpy>=2,<3"
	if ($LASTEXITCODE -ne 0) { throw "pip failed" }
	python (Join-Path $PSScriptRoot "model.py") (Join-Path $out "model.download") $out
	if ($LASTEXITCODE -ne 0) { throw "model preparation failed" }
	Remove-Item (Join-Path $out "model.download")
}
Write-Host "engines ready in $out"
