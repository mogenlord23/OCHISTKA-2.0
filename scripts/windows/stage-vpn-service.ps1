# Готовит файлы службы VPN для установщика: VPN-ядро sing-box (с проверкой SHA-256, tools/vpn-core/fetch.ps1)
# и служба-оболочка (сборка из исходников). Файлы в git не хранятся.
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
& (Join-Path $root 'tools\vpn-core\fetch.ps1')
if ($LASTEXITCODE -ne 0) { throw 'не удалось подготовить VPN-ядро' }
Push-Location $root
try {
    cargo build --release --locked -p vpn-host
    if ($LASTEXITCODE -ne 0) { throw 'не удалось собрать службу VPN' }
}
finally { Pop-Location }
$dest = Join-Path $root 'apps\desktop\src-tauri\resources\vpn-service'
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item (Join-Path $root 'target\release\ochistka-vpn-host.exe') (Join-Path $dest 'ochistka-vpn-host.exe') -Force
Write-Host 'Файлы службы VPN готовы.'
Get-ChildItem $dest -File | ForEach-Object { '{0}  {1}  {2}' -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Length, $_.Name }
