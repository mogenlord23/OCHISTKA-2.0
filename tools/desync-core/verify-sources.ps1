# OCHISTKA: проверка официальных источников обхода блокировок (Zapret 2 / winws2 и WinDivert).
# Скрипт ТОЛЬКО скачивает архивы с официальных страниц выпусков GitHub в папку временных файлов,
# считает SHA-256, размер и состояние подписи драйвера/DLL, печатает сводку и удаляет скачанное.
# Он ничего не запускает и не устанавливает, не отключает защиту Windows, не трогает сеть и реестр.
# Запуск (из папки OCHISTKA):
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\desync-core\verify-sources.ps1

$ErrorActionPreference = 'Stop'
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}

$sources = @(
    @{ Name = 'Zapret 2'; Repo = 'bol-van/zapret2'; License = 'MIT' },
    @{ Name = 'WinDivert'; Repo = 'basil00/WinDivert'; License = 'LGPL v3 / GPL v2' }
)
$work = Join-Path ([IO.Path]::GetTempPath()) ('ochistka-desync-check-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    foreach ($src in $sources) {
        Write-Host ''
        Write-Host ('== {0} ({1}), лицензия: {2}' -f $src.Name, $src.Repo, $src.License)
        try {
            $api = Invoke-RestMethod -Uri ('https://api.github.com/repos/{0}/releases/latest' -f $src.Repo) -UserAgent 'OCHISTKA-verify' -TimeoutSec 25
        } catch {
            Write-Host '  Не удалось получить сведения о последнем выпуске (нет сети или лимит GitHub).'
            continue
        }
        Write-Host ('  Выпуск: {0}, дата: {1}' -f $api.tag_name, $api.published_at)
        foreach ($asset in @($api.assets) | Where-Object { $_.name -match '\.(zip|7z)$' } | Select-Object -First 4) {
            if ($asset.browser_download_url -notmatch '^https://github\.com/') { continue }
            $file = Join-Path $work $asset.name
            Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $file -UseBasicParsing -TimeoutSec 120
            $hash = (Get-FileHash -Path $file -Algorithm SHA256).Hash.ToLower()
            Write-Host ('  Файл: {0}' -f $asset.name)
            Write-Host ('    размер: {0} байт; SHA-256: {1}' -f (Get-Item $file).Length, $hash)
            if ($asset.digest) { Write-Host ('    хэш по данным GitHub: {0}' -f $asset.digest) }
            if ($asset.name -match '\.zip$') {
                $dir = Join-Path $work ($asset.name + '.x')
                Expand-Archive -Path $file -DestinationPath $dir -Force
                foreach ($f in Get-ChildItem -Path $dir -Recurse -File | Where-Object { $_.Name -match '^(winws2?|WinDivert(64)?)\.(exe|dll|sys)$' }) {
                    $sig = Get-AuthenticodeSignature -FilePath $f.FullName
                    $h = (Get-FileHash -Path $f.FullName -Algorithm SHA256).Hash.ToLower()
                    Write-Host ('    {0}: SHA-256 {1}; подпись: {2}' -f $f.Name, $h, $sig.Status)
                }
            }
        }
    }
}
finally {
    Remove-Item -Path $work -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host ''
Write-Host 'Готово: скачанное удалено, ничего не установлено и не запущено. Перешлите этот текст разработчику.'
