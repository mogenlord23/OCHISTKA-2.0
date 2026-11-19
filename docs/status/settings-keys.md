# Компьютер → Настройки и Boost: ключи реестра по параметрам

Дата: 2026-11-19 (круг 4, этап S1). Код: `crates/ochistka-core/src/tweaks_catalog.rs`; тест `every_tweak_has_a_text_in_the_interface_and_a_row_in_the_keys_table` следит, чтобы у каждого параметра была строка здесь и текст в интерфейсе.

Статус для всех строк: **implemented, not verified на Windows** (код собирается под `x86_64-pc-windows-msvc`, логика проверена поддельным реестром; на настоящей Windows эффекты не проверялись). Ключи сверены по документации Microsoft (Group Policy / Windows Privacy) и общеизвестной практике; для недокументированных значений Windows может не поддержать ключ в новой сборке: тогда приложение это заметит при чтении после записи и вернёт прежнее значение.

Правила записи:
- Перед записью прежнее значение сохраняется в `HKCU\Software\OCHISTKA\Backup\<id>` (есть/нет значения, значение). Выключение возвращает его или удаляет значение, если его не было.
- После записи значения читаются обратно. Не совпало: всё откатывается, «включено» не показывается, возникает короткая ошибка.
- Ключи `HKLM\...` пишутся только при запуске приложения от имени администратора. Без прав строка серая с причиной «Нужны права администратора», запись не пытается идти. Повышение прав «на одно действие» пока не сделано.
- Версии: Windows 10 = сборки 10240..19045, Windows 11 = 22000+. В колонке «Версии» указана нижняя/верхняя граница сборки для ключа. Параметр, у которого ни один ключ не подходит текущей сборке, выключен серым с причиной (например, «Есть только в Windows 11»).
- Вступает в силу: «сразу», «Проводник» (после перезапуска Проводника или выхода из системы), «выход» (после выхода из системы), «перезагрузка».

## Внешний вид и рекомендации Windows

| id | Параметр | Ключ | Значение | Версии | Права | Вступает |
|---|---|---|---|---|---|---|
| `transparency` | Убрать прозрачность Windows | `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` `EnableTransparency` | 0 | 10, 11 | пользователь | сразу |
| `spotlight` | Советы и «Интересное» на экране блокировки | `HKCU\...\ContentDeliveryManager` `SubscribedContent-338387/338388/338389/338393/353694/353696/310093Enabled`, `RotatingLockScreenEnabled`, `RotatingLockScreenOverlayEnabled` | 0 | 10, 11 | пользователь | выход |
| `login_blur` | Размытие при входе | `HKLM\SOFTWARE\Policies\Microsoft\Windows\System` `DisableAcrylicBackgroundOnLogon` | 1 | 1903+ (18362) | администратор | сразу |
| `widgets` | Виджеты на панели задач | Windows 11: `HKCU\...\Explorer\Advanced` `TaskbarDa`; Windows 10: `HKCU\...\Feeds` `ShellFeedsTaskbarViewMode` | 0 / 2 | 11: 22000+; 10: 19041..21999 | пользователь | Проводник |
| `fewer_offers` | Меньше персональных предложений | `HKCU\...\Privacy` `TailoredExperiencesWithDiagnosticDataEnabled` | 0 | 10, 11 | пользователь | сразу |
| `extra_tips` | Дополнительные подсказки Windows | `HKCU\...\ContentDeliveryManager` `SoftLandingEnabled`; `HKCU\...\UserProfileEngagement` `ScoobeSystemSettingEnabled` | 0 | 10, 11 | пользователь | сразу |
| `ads` | Рекламные предложения | `HKCU\...\ContentDeliveryManager` `ContentDeliveryAllowed`, `OemPreInstalledAppsEnabled`, `PreInstalledAppsEnabled`, `PreInstalledAppsEverEnabled`, `SilentInstalledAppsEnabled` | 0 | 10, 11 | пользователь | сразу |
| `suggestions` | Рекомендации в «Пуске» | `HKCU\...\ContentDeliveryManager` `SystemPaneSuggestionsEnabled`; `HKCU\...\Explorer\Advanced` `Start_IrisRecommendations` (22621+), `Start_AccountNotifications` (22000+) | 0 | 10, 11 | пользователь | сразу |
| `new_outlook` | Не предлагать новый Outlook | `HKCU\Software\Microsoft\Office\16.0\Outlook\Preferences` `UseNewOutlook`; `HKCU\Software\Policies\Microsoft\Office\16.0\Outlook\Options\General` `DoNewOutlookAutoMigration` | 0 | 10, 11 (нужен Office 2016+ / Microsoft 365) | пользователь | сразу |

## Поиск Windows

| id | Параметр | Ключ | Значение | Версии | Права | Вступает |
|---|---|---|---|---|---|---|
| `web_search` | Веб-подсказки в поиске (исправлено в круге 4: раньше писался только один ключ) | `HKCU\Software\Policies\Microsoft\Windows\Explorer` `DisableSearchBoxSuggestions`; `HKCU\Software\Microsoft\Windows\CurrentVersion\Search` `BingSearchEnabled` | 1; 0 | 10, 11 | пользователь | новые окна поиска сразу; для остального после перезапуска Проводника или выхода |
| `search_highlights` | «Интересное» в поиске | `HKCU\...\SearchSettings` `IsDynamicSearchBoxEnabled` | 0 | 1903+ | пользователь | Проводник |
| `cloud_search` | Облачный поиск | `HKCU\...\SearchSettings` `IsAADCloudSearchEnabled`, `IsMSACloudSearchEnabled` | 0 | 1903+ | пользователь | сразу |

## Конфиденциальность и диагностика

| id | Параметр | Ключ | Значение | Версии | Права | Вступает |
|---|---|---|---|---|---|---|
| `diagnostics` | Минимум диагностики | `HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection` `AllowTelemetry` | 0 (в Home/Pro Windows сводит к минимальному уровню «обязательные») | 10, 11 | администратор | сразу |
| `ad_id` | Рекламный ID | `HKCU\Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo` `Enabled` | 0 | 10, 11 | пользователь | сразу |
| `location` | Геолокация приложений | `HKCU\...\CapabilityAccessManager\ConsentStore\location` `Value` (строка) | `Deny` | 10, 11 | пользователь | сразу |
| `error_reports` | Отчёты об ошибках | `HKCU\Software\Microsoft\Windows\Windows Error Reporting` `Disabled` | 1 | 10, 11 | пользователь | сразу |
| `feedback` | Запросы отзывов | `HKCU\Software\Microsoft\Siuf\Rules` `NumberOfSIUFInPeriod` | 0 | 10, 11 | пользователь | сразу |
| `recall` | Снимки Recall | `HKCU\Software\Policies\Microsoft\Windows\WindowsAI` `DisableAIDataAnalysis` | 1 | 11 24H2+ (26100) | пользователь | сразу |
| `camera` | Камера для приложений | `HKCU\...\ConsentStore\webcam` `Value` | `Deny` | 10, 11 | пользователь | сразу |
| `microphone` | Микрофон для приложений | `HKCU\...\ConsentStore\microphone` `Value` | `Deny` | 10, 11 | пользователь | сразу |
| `contacts` | Контакты для приложений | `HKCU\...\ConsentStore\contacts` `Value` | `Deny` | 10, 11 | пользователь | сразу |

## Microsoft Edge (политики `HKCU\Software\Policies\Microsoft\Edge`)

Политики текущего пользователя Edge учитывает, но в `edge://policy` браузер напишет, что часть параметров «управляется организацией»: так и задумано.

| id | Параметр | Значения | Версии | Права |
|---|---|---|---|---|
| `edge_background` | Не оставлять в фоне | `BackgroundModeEnabled`=0, `StartupBoostEnabled`=0 | 10, 11 | пользователь |
| `edge_telemetry` | Не отправлять сведения о сайтах | `SendSiteInfoToImproveServices`=0, `MetricsReportingEnabled`=0 | 10, 11 | пользователь |
| `edge_ads` | Персональная реклама | `PersonalizationReportingEnabled`=0 | 10, 11 | пользователь |
| `edge_suggest` | Предложения поиска | `SearchSuggestEnabled`=0 | 10, 11 | пользователь |
| `edge_passwords` | Не сохранять новые пароли | `PasswordManagerEnabled`=0 | 10, 11 | пользователь |
| `edge_sidebar` | Боковая панель и Copilot | `HubsSidebarEnabled`=0 | 10, 11 | пользователь |

## Системные компоненты и облако

| id | Параметр | Ключ | Значение | Версии | Права | Вступает |
|---|---|---|---|---|---|---|
| `cortana` | Cortana (если есть) | `HKCU\...\Search` `CortanaConsent`; `HKCU\...\Explorer\Advanced` `ShowCortanaButton` | 0 | только Windows 10 (до 21999) | пользователь | Проводник |
| `copilot` | Windows Copilot | `HKCU\Software\Policies\Microsoft\Windows\WindowsCopilot` `TurnOffWindowsCopilot`; `HKCU\...\Explorer\Advanced` `ShowCopilotButton` | 1; 0 | Windows 11 22H2+ (22621) | пользователь | Проводник |
| `onedrive_sync` | Синхронизация OneDrive | `HKLM\SOFTWARE\Policies\Microsoft\Windows\OneDrive` `DisableFileSyncNGSC` | 1 | 10, 11 | администратор | выход |
| `settings_sync` | Синхронизация параметров профиля | `HKLM\SOFTWARE\Policies\Microsoft\Windows\SettingSync` `DisableSettingSync`=2, `DisableSettingSyncUserOverride`=1 | см. колонку | 10, 11 | администратор | выход |

## Обновления Windows

| id | Параметр | Ключ | Значение | Версии | Права |
|---|---|---|---|---|---|
| `update_ask` | Спрашивать перед загрузкой | `HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU` `NoAutoUpdate`=0, `AUOptions`=2 | см. колонку | 10, 11 | администратор |
| `no_auto_reboot` | Не перезагружать при активном сеансе | тот же ключ `NoAutoRebootWithLoggedOnUsers`=1 | 1 | 10, 11 | администратор |

## Boost

| id | Параметр | Ключ | Значение | Версии | Права | Вступает |
|---|---|---|---|---|---|---|
| `game_dvr` | Убрать фоновую запись игр (баг круга 4: раньше менялся один ключ, Windows 11 и приложение захвата его игнорировали) | `HKCU\System\GameConfigStore` `GameDVR_Enabled`; `HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR` `AppCaptureEnabled`, `HistoricalCaptureEnabled` | 0 | 10 (1809+), 11 | пользователь | сразу |
| `game_dvr_policy` | Запретить запись игр политикой | `HKLM\SOFTWARE\Policies\Microsoft\Windows\GameDVR` `AllowGameDVR` | 0 | 10, 11 | администратор | сразу |
| `game_mode` | Игровой режим Windows | `HKCU\Software\Microsoft\GameBar` `AutoGameModeEnabled`, `AllowAutoGameMode` | 1 | 10, 11 | пользователь | сразу |
| `power_throttling` | Без ограничения фоновой мощности | `HKLM\SYSTEM\CurrentControlSet\Control\Power\PowerThrottling` `PowerThrottlingOff` | 1 | 10, 11 | администратор | для новых процессов |
| `ultimate` | Ultimate Performance | схема питания `e9a42b02-d5df-448d-aa00-03f14749eb61` через `C:\Windows\System32\powercfg.exe` (`/list`, при отсутствии `/duplicatescheme guid guid`, `/setactive`); прежняя схема в `HKCU\Software\OCHISTKA\Backup\boost` `PowerScheme` | схема | 10, 11 | пользователь | сразу |
| `hags` | Планирование GPU (HAGS) | `HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers` `HwSchMode` | 2 | 2004+ (19041), нужны поддерживаемые видеокарта и драйвер | администратор | перезагрузка |
| `vbs_off` | Игровой режим без VBS/HVCI (опасно) | `HKLM\SYSTEM\CurrentControlSet\Control\DeviceGuard` `EnableVirtualizationBasedSecurity`; `...\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity` `Enabled` | 0 | 10, 11 | администратор | перезагрузка |

Профили Boost (Базовый / Игровой / Максимум) взаимоисключающие и независимы от строк выше: «Базовый» = `game_mode` + `game_dvr`; «Игровой» добавляет схему «Высокая производительность» (`8c5e7fda-...`); «Максимум» добавляет схему Ultimate (тот же механизм, что у `ultimate`).

## Информационный блок «Активация Windows»

Не переключатель. Статус: `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -NonInteractive -Command` с фиксированным текстом `Get-CimInstance SoftwareLicensingProduct` (WMI, `LicenseStatus`: 1 = активирована, 0 = нет, 2-4 и 6 = льготный период, 5 = уведомление). Кнопка «Открыть параметры активации» запускает `C:\Windows\explorer.exe ms-settings:activation`. Не проверено на Windows.
