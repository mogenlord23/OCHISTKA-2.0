#requires -version 5
<#
    Fetches the official signed Windows build of ClamAV and unpacks only what the
    application needs: libclamav.dll with its DLL dependencies, the import library
    and the public headers. clamscan.exe / freshclam.exe / clamd.exe are never extracted.

    Provenance is pinned here: version, release URL, archive size and SHA-256.
    The hash is the one GitHub reports for the release asset (API field "digest").

    Usage:  powershell -File tools\clamav-win\fetch.ps1 [-Force]
#>
[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.IO.Compression.FileSystem | Out-Null

$Version = '1.4.6'
$Sha256 = '57B6FD1D60CD87BAFE800F97407ECDEF0576D36B3900B8B7ABCFBBABE88295FD'
$ExpectedSize = 191947200
$Url = "https://github.com/Cisco-Talos/clamav/releases/download/clamav-$Version/clamav-$Version.win.x64.zip"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$DownloadDir = Join-Path $Root '_download'
$Archive = Join-Path $DownloadDir "clamav-$Version.win.x64.zip"
$Marker = Join-Path $Root 'VERSION.txt'
$BinDir = Join-Path $Root 'bin'
$LibDir = Join-Path $Root 'lib'
$IncludeDir = Join-Path $Root 'include'

function Write-Step([string]$Text) {
    Write-Host "[clamav] $Text"
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToUpperInvariant()
}

New-Item -ItemType Directory -Force -Path $DownloadDir, $BinDir, $LibDir, $IncludeDir | Out-Null

# 1. Archive: reuse a verified copy, otherwise download to the same pinned URL.
$needDownload = $true
if ((Test-Path -LiteralPath $Archive) -and -not $Force) {
    $size = (Get-Item -LiteralPath $Archive).Length
    if ($size -eq $ExpectedSize) {
        $hash = Get-Sha256 $Archive
        if ($hash -eq $Sha256) {
            Write-Step "archive already verified: $Archive"
            $needDownload = $false
        }
        else {
            Write-Step "cached archive has a different hash, downloading again"
        }
    }
}

if ($needDownload) {
    Write-Step "downloading $Url"
    $curl = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($curl) {
        & $curl.Source -sS -L -C - --retry 3 --retry-delay 2 -o $Archive $Url
        if ($LASTEXITCODE -ne 0) {
            throw "download failed (curl exit $LASTEXITCODE). The pinned release is clamav-$Version; do not switch to another version silently."
        }
    }
    else {
        Invoke-WebRequest -Uri $Url -OutFile $Archive -UseBasicParsing
    }
    $size = (Get-Item -LiteralPath $Archive).Length
    if ($size -ne $ExpectedSize) {
        throw "archive size $size does not match the pinned $ExpectedSize for clamav-$Version"
    }
    $hash = Get-Sha256 $Archive
    if ($hash -ne $Sha256) {
        throw "SHA-256 mismatch: got $hash, expected $Sha256. Nothing was unpacked."
    }
}

# 2. Unpack only the files the application ships.
$prefix = "clamav-$Version.win.x64/"
$zip = [System.IO.Compression.ZipFile]::OpenRead($Archive)
try {
    $copied = 0
    foreach ($entry in $zip.Entries) {
        if ($entry.FullName.EndsWith('/')) { continue }
        $relative = $entry.FullName.Substring($prefix.Length)
        $target = $null
        if ($entry.FullName.StartsWith($prefix + 'include/')) {
            $target = Join-Path $IncludeDir (Split-Path -Leaf $relative)
        }
        elseif ($relative -eq 'clamav.lib') {
            $target = Join-Path $LibDir 'clamav.lib'
        }
        elseif ($relative -eq 'COPYING.txt') {
            $target = Join-Path $Root 'COPYING.txt'
        }
        elseif ($relative -like 'COPYING/*') {
            $target = Join-Path (Join-Path $Root 'COPYING') (Split-Path -Leaf $relative)
        }
        elseif ($relative -match '^[^/]+\.dll$') {
            $target = Join-Path $BinDir (Split-Path -Leaf $relative)
        }
        if (-not $target) { continue }
        $parent = Split-Path -Parent $target
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $target, $true)
        $copied++
    }
    Write-Step "unpacked $copied files into $Root"
}
finally {
    $zip.Dispose()
}

if (-not (Test-Path -LiteralPath (Join-Path $BinDir 'libclamav.dll'))) {
    throw "libclamav.dll is missing after unpacking; the archive layout changed, do not continue silently."
}

# 3. Report what was pinned, including the version printed by the archive itself.
$versionHeader = Join-Path $IncludeDir 'clamav-version.h'
$libVersion = 'unknown'
if (Test-Path -LiteralPath $versionHeader) {
    $text = Get-Content -LiteralPath $versionHeader -Raw
    $m = [regex]::Match($text, '#define\s+CLAMAV_VERSION\s+"([^"]+)"')
    if ($m.Success) { $libVersion = $m.Groups[1].Value }
}

$lines = @(
    "source: $Url"
    "archive sha256: $Sha256"
    "archive bytes: $ExpectedSize"
    "clamav version: $libVersion"
    "fetched at: $((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))"
)
$lines | Set-Content -LiteralPath $Marker -Encoding ASCII
$lines | ForEach-Object { Write-Step $_ }

Write-Step "done: DLLs in $BinDir, import library in $LibDir, headers in $IncludeDir"
