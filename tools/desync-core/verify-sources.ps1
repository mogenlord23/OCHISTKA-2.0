# OCHISTKA: проверка официальных источников обхода блокировок (Zapret 2 / winws2 и WinDivert).
# Скрипт НИЧЕГО НЕ СКАЧИВАЕТ и не открывает чужие файлы: он только читает с официальной страницы
# выпусков GitHub список файлов, их размер и контрольную сумму SHA-256, которую GitHub считает сам.
# Защита Windows не отключается и не обходится. Ничего не запускается и не устанавливается.
# Запуск (из папки OCHISTKA):
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\desync-core\verify-sources.ps1

$ErrorActionPreference = 'Stop'
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}

$sources = @(
    @{ Name = 'Zapret 2'; Repo = 'bol-van/zapret2'; License = 'MIT' },
    @{ Name = 'WinDivert'; Repo = 'basil00/WinDivert'; License = 'LGPL v3 / GPL v2' }
)
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
    foreach ($asset in @($api.assets)) {
        if ($asset.browser_download_url -notmatch '^https://github\.com/') { continue }
        $digest = if ($asset.digest) { [string]$asset.digest } else { '(GitHub не сообщил)' }
        Write-Host ('  Файл: {0}' -f $asset.name)
        Write-Host ('    размер: {0} байт; контрольная сумма по данным GitHub: {1}' -f $asset.size, $digest)
    }
}
Write-Host ''
Write-Host 'Готово: ничего не скачано, не установлено и не запущено. Перешлите этот текст разработчику.'
