# Скачивает SonicDPI v0.2.2, проверяет SHA-256 архива и файла движка, кладёт нужные файлы в папку
# назначения (sonicdpi.exe сохраняется как ochistkadpi.exe, код не изменён). Файлы в git не хранятся.
param([Parameter(Mandatory = $true)][string]$Destination)
$ErrorActionPreference = 'Stop'
$url = 'https://github.com/by-sonic/sonicdpi/releases/download/v0.2.2/SonicDPI-windows-x64.zip'
$archiveSha = 'ec1e9f32315bdab1989d8e78bb3d3ad6975700fe83546139eb6f21e132991fc0'
$engineSha = '3d6451021f390f8302b3aa1d790b86e626d7f925f7e1a2ba9c51ec91afa35351'
$zip = Join-Path ([System.IO.Path]::GetTempPath()) 'SonicDPI-windows-x64.zip'
Invoke-WebRequest -Uri $url -OutFile $zip
$actual = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLowerInvariant()
if ($actual -ne $archiveSha) { throw "SHA-256 архива не совпал: ожидался $archiveSha, получен $actual" }
Write-Host "SHA-256 архива совпал: $actual"
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
$wanted = @('sonicdpi.exe', 'WinDivert.dll', 'WinDivert64.sys', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY_LICENSES.md')
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($zip)
try {
    foreach ($name in $wanted) {
        $entry = $archive.Entries | Where-Object { $_.FullName -eq $name } | Select-Object -First 1
        if (-not $entry) { throw "В архиве нет файла: $name" }
        $target = if ($name -eq 'sonicdpi.exe') { 'ochistkadpi.exe' } else { $name }
        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $Destination $target), $true)
    }
}
finally { $archive.Dispose() }
$exeHash = (Get-FileHash (Join-Path $Destination 'ochistkadpi.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
if ($exeHash -ne $engineSha) { throw "SHA-256 файла движка не совпал: ожидался $engineSha, получен $exeHash" }
Write-Host "SHA-256 файла движка совпал (ochistkadpi.exe): $exeHash"
Get-ChildItem $Destination -File | ForEach-Object { '{0}  {1}  {2}' -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Length, $_.Name }
