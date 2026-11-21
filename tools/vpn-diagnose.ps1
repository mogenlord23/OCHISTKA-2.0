# OCHISTKA: безопасная диагностика ссылки VPN-подписки.
# Ссылка вводится скрыто и нигде не сохраняется. На экран выводятся только сводные числа:
# код ответа, тип данных, формат и сколько серверов каких видов. Ни ссылка, ни ключи,
# ни адреса серверов, ни содержимое ответа не печатаются и не записываются.
# Запуск (из папки OCHISTKA):  powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\vpn-diagnose.ps1

$ErrorActionPreference = 'Stop'
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}

$secure = Read-Host 'Вставьте ссылку подписки (ввод скрыт) и нажмите Enter' -AsSecureString
$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
try { $url = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr).Trim() }
finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr) }

if ($url -notmatch '^https://[^\s/]+') {
    Write-Host 'Это не ссылка https:// (нужна ссылка, которая начинается с https://).'
    exit 1
}

# Те же имена клиента, что использует OCHISTKA.
$agents = @('v2rayN/7.0 OCHISTKA', 'v2rayTun/1.0 OCHISTKA')
$knownTransports = @('tcp', 'raw', 'ws', 'websocket', 'grpc', 'http', 'h2', 'httpupgrade', 'xhttp', 'splithttp', 'kcp', 'quic')
$supportedTransports = @('tcp', 'raw', 'ws', 'websocket', 'grpc', 'http', 'h2', 'httpupgrade')
$knownSecurity = @('none', 'tls', 'reality', 'xtls')

function Add-Count($table, $key) {
    if ($table.ContainsKey($key)) { $table[$key] = $table[$key] + 1 } else { $table[$key] = 1 }
}

function Show-Counts($title, $table) {
    if ($table.Count -eq 0) { return }
    $parts = $table.GetEnumerator() | Sort-Object Name | ForEach-Object { '{0}: {1}' -f $_.Name, $_.Value }
    Write-Host ('  {0}: {1}' -f $title, ($parts -join ', '))
}

function Try-Base64($text) {
    $t = ($text -replace '\s', '').Replace('-', '+').Replace('_', '/')
    if ($t.Length -lt 8 -or $t -notmatch '^[A-Za-z0-9+/]+=*$') { return $null }
    while ($t.Length % 4 -ne 0) { $t += '=' }
    try { return [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($t)) } catch { return $null }
}

function Describe-Body($text, $allowBase64) {
    $body = $text.Trim([char]0xFEFF).Trim()
    if ($body.Length -eq 0) { Write-Host '  Формат: пустой ответ'; return }
    if ($body.StartsWith('<')) { Write-Host '  Формат: веб-страница (HTML), а не подписка'; return }
    if ($body.StartsWith('{') -or $body.StartsWith('[')) {
        $json = $null
        try { $json = $body | ConvertFrom-Json } catch {}
        if ($null -eq $json) { Write-Host '  Формат: JSON, но он не читается'; return }
        $configs = @($json)
        $protocols = @{}; $nets = @{}; $configCount = 0
        foreach ($c in $configs) {
            $configCount++
            foreach ($o in @($c.outbounds)) {
                $p = if ($o.protocol) { [string]$o.protocol } elseif ($o.type) { [string]$o.type } else { 'другое' }
                Add-Count $protocols $p.ToLower()
                $n = $null
                if ($o.streamSettings -and $o.streamSettings.network) { $n = [string]$o.streamSettings.network }
                elseif ($o.transport -and $o.transport.type) { $n = [string]$o.transport.type }
                if ($n) { if ($knownTransports -contains $n.ToLower()) { Add-Count $nets $n.ToLower() } else { Add-Count $nets 'другое' } }
            }
        }
        $kind = if ($body -match '"protocol"') { 'Xray JSON' } else { 'sing-box JSON' }
        Write-Host ('  Формат: {0} (конфигов: {1})' -f $kind, $configCount)
        Show-Counts 'Протоколы' $protocols
        Show-Counts 'Транспорты' $nets
        return
    }
    if ($body -match '(?m)^\s*proxies\s*:') {
        $types = @{}
        foreach ($m in [regex]::Matches($body, '(?m)type\s*:\s*[''"]?([A-Za-z0-9]+)')) { Add-Count $types $m.Groups[1].Value.ToLower() }
        Write-Host '  Формат: Clash YAML (OCHISTKA пока не умеет его читать)'
        Show-Counts 'Типы' $types
        return
    }
    $lines = $body -split "`r?`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith('#') }
    $linkLines = @($lines | Where-Object { $_ -match '^[A-Za-z][A-Za-z0-9+.-]{1,14}://' })
    if ($linkLines.Count -gt 0) {
        $schemes = @{}; $nets = @{}; $secs = @{}; $unsupportedNet = 0; $supportedNet = 0
        foreach ($l in $linkLines) {
            $s = ($l -split '://')[0].ToLower()
            Add-Count $schemes $s
            if ($s -in @('vless', 'trojan')) {
                $q = ''
                if ($l -match '\?([^#]*)') { $q = $Matches[1] }
                $net = 'tcp'; $sec = 'none'
                foreach ($pair in ($q -split '&')) {
                    $kv = $pair -split '=', 2
                    if ($kv.Count -eq 2) {
                        if ($kv[0] -eq 'type') { $net = $kv[1].ToLower() }
                        if ($kv[0] -eq 'security') { $sec = $kv[1].ToLower() }
                    }
                }
                if ($knownTransports -contains $net) { Add-Count $nets $net } else { Add-Count $nets 'другое' }
                if ($knownSecurity -contains $sec) { Add-Count $secs $sec } else { Add-Count $secs 'другое' }
                if ($supportedTransports -contains $net) { $supportedNet++ } else { $unsupportedNet++ }
            }
        }
        Write-Host ('  Формат: список ссылок (строк со ссылками: {0})' -f $linkLines.Count)
        Show-Counts 'Схемы' $schemes
        Show-Counts 'Транспорты (vless/trojan)' $nets
        Show-Counts 'Защита (vless/trojan)' $secs
        if ($unsupportedNet -gt 0) {
            Write-Host ('  ВНИМАНИЕ: {0} ссылок используют транспорт, который ядро sing-box не поддерживает; они будут пропущены.' -f $unsupportedNet)
        }
        return
    }
    if ($allowBase64) {
        $decoded = Try-Base64 $body
        if ($null -ne $decoded -and $decoded.Trim().Length -gt 0) {
            Write-Host '  Ответ закодирован в base64, внутри:'
            Describe-Body $decoded $false
            return
        }
    }
    Write-Host ('  Формат: не распознан (длина ответа {0} символов)' -f $body.Length)
}

Write-Host ''
Write-Host 'Проверяю подписку (ссылка и содержимое нигде не показываются и не сохраняются)...'
$i = 0
foreach ($agent in $agents) {
    $i++
    Write-Host ''
    Write-Host ('Попытка {0} (имя клиента: {1})' -f $i, ($agent -split ' ')[0])
    try {
        $r = Invoke-WebRequest -Uri $url -UseBasicParsing -UserAgent $agent -TimeoutSec 25 -MaximumRedirection 5
        $bytes = $r.RawContentStream.ToArray()
        $ctype = [string]$r.Headers['Content-Type']
        $ctype = ($ctype -split ';')[0]
        Write-Host ('  Ответ: код {0}, тип данных "{1}", размер {2} байт' -f $r.StatusCode, $ctype, $bytes.Length)
        Describe-Body ([Text.Encoding]::UTF8.GetString($bytes)) $true
    }
    catch {
        $resp = $_.Exception.Response
        if ($null -ne $resp) {
            Write-Host ('  Сервис ответил отказом: код {0}' -f [int]$resp.StatusCode)
        } else {
            $kind = $_.Exception.GetType().Name
            Write-Host ('  Не удалось связаться с сервисом (тип ошибки: {0})' -f $kind)
        }
    }
}
Write-Host ''
Write-Host 'Готово. Перешлите разработчику только этот текст выше: в нём нет ссылки и ключей.'
