# OCHISTKA: безопасная проверка настоящего VPN-подключения на Windows.
# Скрипт ничего не меняет: он только читает системные настройки прокси и список процессов sing-box
# до подключения, во время него и после отключения. Подписки, ключи и адреса серверов не читаются
# и не печатаются. Запуск (из папки OCHISTKA):
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\vpn-check-windows.ps1

$ErrorActionPreference = 'Stop'
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'

function Get-State($title) {
    $p = Get-ItemProperty -Path $key
    $enable = if ($null -ne $p.ProxyEnable) { [int]$p.ProxyEnable } else { 0 }
    $server = if ($p.ProxyServer) { [string]$p.ProxyServer } else { '(не задан)' }
    $pac = if ($p.AutoConfigURL) { 'задан' } else { 'не задан' }
    $cores = @(Get-Process -Name 'sing-box' -ErrorAction SilentlyContinue)
    Write-Host ('{0}' -f $title)
    Write-Host ('  Системный прокси включён: {0}; адрес прокси: {1}; сценарий автонастройки: {2}' -f $enable, $server, $pac)
    Write-Host ('  Процессов sing-box: {0}' -f $cores.Count)
    return [pscustomobject]@{ Enable = $enable; Server = $server; Pac = $pac; Cores = $cores.Count }
}

Write-Host ''
Write-Host 'Шаг 1. Запустите OCHISTKA (npm run dev), импортируйте подписку и выберите сервер.'
Write-Host '       VPN пока НЕ подключайте.'
[void](Read-Host 'Когда готово, нажмите Enter')
$before = Get-State 'ДО подключения:'

Write-Host ''
Write-Host 'Шаг 2. В OCHISTKA нажмите кнопку подключения и дождитесь результата на экране.'
[void](Read-Host 'Когда результат появился, нажмите Enter')
$during = Get-State 'ВО ВРЕМЯ подключения:'
if ($during.Enable -eq 1 -and $during.Server -match '^127\.0\.0\.1:\d+$') {
    $proxy = 'http://' + $during.Server
    try {
        $r = Invoke-WebRequest -Uri 'https://www.gstatic.com/generate_204' -Proxy $proxy -UseBasicParsing -TimeoutSec 15
        Write-Host ('  Проверка трафика через прокси: ответ {0} (данные проходят)' -f $r.StatusCode)
    } catch {
        Write-Host ('  Проверка трафика через прокси: НЕ прошла (тип ошибки: {0})' -f $_.Exception.GetType().Name)
    }
} else {
    Write-Host '  Прокси не включён: подключение не выполнено или завершилось ошибкой.'
}

Write-Host ''
Write-Host 'Шаг 3. В OCHISTKA нажмите кнопку отключения.'
[void](Read-Host 'Когда отключилось, нажмите Enter')
$after = Get-State 'ПОСЛЕ отключения:'

Write-Host ''
$restored = ($after.Enable -eq $before.Enable) -and ($after.Server -eq $before.Server) -and ($after.Pac -eq $before.Pac)
Write-Host ('Прежние настройки прокси возвращены: {0}' -f $(if ($restored) { 'ДА' } else { 'НЕТ' }))
Write-Host ('Осталось процессов sing-box: {0} (должно быть 0)' -f $after.Cores)
Write-Host ''
Write-Host 'Готово. Перешлите разработчику только этот текст: в нём нет ссылок и ключей.'
