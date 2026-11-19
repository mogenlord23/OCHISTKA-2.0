//! Каталог настроек Windows и Boost: для каждого параметра ключи реестра, значение «включено»
//! и диапазон сборок Windows, где ключ действует. Таблица «параметр → ключ → версии» для людей:
//! `docs/status/settings-keys.md` (тест сверяет, что каждый id там упомянут).
//! Ключи без приставки лежат в HKCU, с приставкой `HKLM\` — в HKLM (нужны права администратора).

/// Windows 10 1903 и Windows 11 24H2 и пр.: номера сборок (CurrentBuildNumber).
pub const WIN10_1809: u32 = 17763;
pub const WIN10_1903: u32 = 18362;
pub const WIN10_2004: u32 = 19041;
pub const WIN11: u32 = 22000;
pub const WIN11_22H2: u32 = 22621;
pub const WIN11_24H2: u32 = 26100;
pub const ANY: u32 = 0;
pub const NEVER_ENDS: u32 = u32::MAX;
/// Последняя сборка Windows 10 (до Windows 11).
pub const LAST_WIN10: u32 = WIN11 - 1;

/// Раздел интерфейса, которому принадлежит настройка.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Settings,
    Boost,
}

impl Group {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "settings" => Some(Self::Settings),
            "boost" => Some(Self::Boost),
            _ => None,
        }
    }
}

/// Когда изменение вступает в силу.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartKind {
    None,
    Explorer,
    SignOut,
    Reboot,
}

impl RestartKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Explorer => "explorer",
            Self::SignOut => "signOut",
            Self::Reboot => "reboot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Val {
    Dword(u32),
    Text(&'static str),
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub key: &'static str,
    pub name: &'static str,
    /// Значение, которое включает настройку.
    pub value: Val,
    pub min_build: u32,
    pub max_build: u32,
}

impl Entry {
    pub const fn is_hklm(&self) -> bool {
        // Приставка `HKLM\` из 5 байт.
        let key = self.key.as_bytes();
        key.len() > 5
            && key[0] == b'H'
            && key[1] == b'K'
            && key[2] == b'L'
            && key[3] == b'M'
            && key[4] == b'\\'
    }

    pub fn applies_to(&self, build: Option<u32>) -> bool {
        build.is_none_or(|b| b >= self.min_build && b <= self.max_build)
    }

    const fn from(mut self, build: u32) -> Self {
        self.min_build = build;
        self
    }

    const fn until(mut self, build: u32) -> Self {
        self.max_build = build;
        self
    }
}

const fn d(key: &'static str, name: &'static str, value: u32) -> Entry {
    Entry {
        key,
        name,
        value: Val::Dword(value),
        min_build: ANY,
        max_build: NEVER_ENDS,
    }
}

const fn t(key: &'static str, name: &'static str, value: &'static str) -> Entry {
    Entry {
        key,
        name,
        value: Val::Text(value),
        min_build: ANY,
        max_build: NEVER_ENDS,
    }
}

/// Политика компьютера, которая управляет тем же параметром и сильнее значения пользователя.
#[derive(Clone, Copy)]
pub struct Policy {
    /// Ключ HKLM или HKCU (по тем же правилам, что в `Entry`).
    pub key: &'static str,
    pub name: &'static str,
    /// Значение политики, при котором параметр уже действует (настройка «включена» политикой).
    pub on: u32,
}

pub struct TweakDef {
    pub id: &'static str,
    pub group: Group,
    pub restart: RestartKind,
    /// Опасное: включается только после явного подтверждения.
    pub dangerous: bool,
    /// Схема электропитания вместо реестра (GUID).
    pub power_scheme: Option<&'static str>,
    pub entries: &'static [Entry],
    /// Политики, которые перекрывают значения `entries`. Если политика задана, параметр
    /// управляется ею: изменить его из программы нельзя, и интерфейс объясняет почему.
    pub policies: &'static [Policy],
}

impl TweakDef {
    const fn policies(mut self, policies: &'static [Policy]) -> Self {
        self.policies = policies;
        self
    }
}

pub const ULTIMATE_SCHEME: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
pub const HIGH_SCHEME: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";

const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const ADV: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const SEARCH: &str = r"Software\Microsoft\Windows\CurrentVersion\Search";
const SEARCH_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\SearchSettings";
const EDGE: &str = r"Software\Policies\Microsoft\Edge";
const UPDATE_AU: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU";
const GAME_DVR: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
const DEVICE_GUARD: &str = r"HKLM\SYSTEM\CurrentControlSet\Control\DeviceGuard";

const fn consent(key: &'static str) -> Entry {
    t(key, "Value", "Deny")
}

const fn cdm(name: &'static str) -> Entry {
    d(CDM, name, 0)
}

const fn def(
    id: &'static str,
    group: Group,
    restart: RestartKind,
    entries: &'static [Entry],
) -> TweakDef {
    TweakDef {
        id,
        group,
        restart,
        dangerous: false,
        power_scheme: None,
        entries,
        policies: &[],
    }
}

use Group::{Boost, Settings};
use RestartKind::{Explorer, None as NoRestart, Reboot, SignOut};

pub const TWEAKS: &[TweakDef] = &[
    // Внешний вид и рекомендации Windows
    def(
        "transparency",
        Settings,
        NoRestart,
        &[d(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "EnableTransparency",
            0,
        )],
    ),
    def(
        "spotlight",
        Settings,
        SignOut,
        &[
            cdm("SubscribedContent-338387Enabled"),
            cdm("SubscribedContent-338388Enabled"),
            cdm("SubscribedContent-338389Enabled"),
            cdm("SubscribedContent-338393Enabled"),
            cdm("SubscribedContent-353694Enabled"),
            cdm("SubscribedContent-353696Enabled"),
            cdm("SubscribedContent-310093Enabled"),
            cdm("RotatingLockScreenEnabled"),
            cdm("RotatingLockScreenOverlayEnabled"),
        ],
    ),
    def(
        "login_blur",
        Settings,
        NoRestart,
        &[d(
            r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System",
            "DisableAcrylicBackgroundOnLogon",
            1,
        )
        .from(WIN10_1903)],
    ),
    def(
        "widgets",
        Settings,
        Explorer,
        &[
            d(ADV, "TaskbarDa", 0).from(WIN11),
            d(
                r"Software\Microsoft\Windows\CurrentVersion\Feeds",
                "ShellFeedsTaskbarViewMode",
                2,
            )
            .from(WIN10_2004)
            .until(LAST_WIN10),
        ],
    ),
    def(
        "fewer_offers",
        Settings,
        NoRestart,
        &[d(
            r"Software\Microsoft\Windows\CurrentVersion\Privacy",
            "TailoredExperiencesWithDiagnosticDataEnabled",
            0,
        )],
    ),
    def(
        "extra_tips",
        Settings,
        NoRestart,
        &[
            cdm("SoftLandingEnabled"),
            d(
                r"Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement",
                "ScoobeSystemSettingEnabled",
                0,
            ),
        ],
    ),
    def(
        "ads",
        Settings,
        NoRestart,
        &[
            cdm("ContentDeliveryAllowed"),
            cdm("OemPreInstalledAppsEnabled"),
            cdm("PreInstalledAppsEnabled"),
            cdm("PreInstalledAppsEverEnabled"),
            cdm("SilentInstalledAppsEnabled"),
        ],
    ),
    def(
        "suggestions",
        Settings,
        NoRestart,
        &[
            cdm("SystemPaneSuggestionsEnabled"),
            d(ADV, "Start_IrisRecommendations", 0).from(WIN11_22H2),
            d(ADV, "Start_AccountNotifications", 0).from(WIN11),
            d(ADV, "Start_TrackProgs", 0).from(WIN11_22H2),
        ],
    )
    .policies(&[Policy {
        key: r"HKLM\SOFTWARE\Policies\Microsoft\Windows\CloudContent",
        name: "DisableWindowsConsumerFeatures",
        on: 1,
    }]),
    def(
        "new_outlook",
        Settings,
        NoRestart,
        &[
            d(
                r"Software\Microsoft\Office\16.0\Outlook\Preferences",
                "UseNewOutlook",
                0,
            ),
            d(
                r"Software\Policies\Microsoft\Office\16.0\Outlook\Options\General",
                "DoNewOutlookAutoMigration",
                0,
            ),
        ],
    ),
    // Поиск Windows
    def(
        "web_search",
        Settings,
        Explorer,
        &[
            d(
                r"Software\Policies\Microsoft\Windows\Explorer",
                "DisableSearchBoxSuggestions",
                1,
            ),
            d(SEARCH, "BingSearchEnabled", 0),
        ],
    ),
    def(
        "search_highlights",
        Settings,
        Explorer,
        &[d(SEARCH_SETTINGS, "IsDynamicSearchBoxEnabled", 0).from(WIN10_1903)],
    ),
    def(
        "cloud_search",
        Settings,
        NoRestart,
        &[
            d(SEARCH_SETTINGS, "IsAADCloudSearchEnabled", 0).from(WIN10_1903),
            d(SEARCH_SETTINGS, "IsMSACloudSearchEnabled", 0).from(WIN10_1903),
        ],
    ),
    // Конфиденциальность и диагностика
    def(
        "diagnostics",
        Settings,
        NoRestart,
        &[d(
            r"HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection",
            "AllowTelemetry",
            0,
        )],
    ),
    def(
        "ad_id",
        Settings,
        NoRestart,
        &[d(
            r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
            "Enabled",
            0,
        )],
    )
    .policies(&[Policy {
        key: r"HKLM\SOFTWARE\Policies\Microsoft\Windows\AdvertisingInfo",
        name: "DisabledByGroupPolicy",
        on: 1,
    }]),
    def(
        "location",
        Settings,
        NoRestart,
        &[consent(
            r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location",
        )],
    ),
    def(
        "error_reports",
        Settings,
        NoRestart,
        &[d(
            r"Software\Microsoft\Windows\Windows Error Reporting",
            "Disabled",
            1,
        )],
    ),
    def(
        "feedback",
        Settings,
        NoRestart,
        &[d(
            r"Software\Microsoft\Siuf\Rules",
            "NumberOfSIUFInPeriod",
            0,
        )],
    ),
    def(
        "recall",
        Settings,
        NoRestart,
        &[d(
            r"Software\Policies\Microsoft\Windows\WindowsAI",
            "DisableAIDataAnalysis",
            1,
        )
        .from(WIN11_24H2)],
    ),
    def(
        "camera",
        Settings,
        NoRestart,
        &[consent(
            r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\webcam",
        )],
    ),
    def(
        "microphone",
        Settings,
        NoRestart,
        &[consent(
            r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone",
        )],
    ),
    def(
        "contacts",
        Settings,
        NoRestart,
        &[consent(
            r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\contacts",
        )],
    ),
    // Microsoft Edge (политики текущего пользователя)
    def(
        "edge_background",
        Settings,
        NoRestart,
        &[
            d(EDGE, "BackgroundModeEnabled", 0),
            d(EDGE, "StartupBoostEnabled", 0),
        ],
    ),
    def(
        "edge_telemetry",
        Settings,
        NoRestart,
        &[
            d(EDGE, "SendSiteInfoToImproveServices", 0),
            d(EDGE, "MetricsReportingEnabled", 0),
        ],
    ),
    def(
        "edge_ads",
        Settings,
        NoRestart,
        &[d(EDGE, "PersonalizationReportingEnabled", 0)],
    ),
    def(
        "edge_suggest",
        Settings,
        NoRestart,
        &[d(EDGE, "SearchSuggestEnabled", 0)],
    ),
    def(
        "edge_passwords",
        Settings,
        NoRestart,
        &[d(EDGE, "PasswordManagerEnabled", 0)],
    ),
    def(
        "edge_sidebar",
        Settings,
        NoRestart,
        &[d(EDGE, "HubsSidebarEnabled", 0)],
    ),
    // Системные компоненты и облако
    def(
        "cortana",
        Settings,
        Explorer,
        &[
            d(SEARCH, "CortanaConsent", 0).until(LAST_WIN10),
            d(ADV, "ShowCortanaButton", 0).until(LAST_WIN10),
        ],
    ),
    def(
        "copilot",
        Settings,
        Explorer,
        &[
            d(
                r"Software\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
                1,
            )
            .from(WIN11_22H2),
            d(ADV, "ShowCopilotButton", 0).from(WIN11_22H2),
        ],
    ),
    def(
        "onedrive_sync",
        Settings,
        SignOut,
        &[d(
            r"HKLM\SOFTWARE\Policies\Microsoft\Windows\OneDrive",
            "DisableFileSyncNGSC",
            1,
        )],
    ),
    def(
        "settings_sync",
        Settings,
        SignOut,
        &[
            d(
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows\SettingSync",
                "DisableSettingSync",
                2,
            ),
            d(
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows\SettingSync",
                "DisableSettingSyncUserOverride",
                1,
            ),
        ],
    ),
    // Обновления Windows
    def(
        "update_ask",
        Settings,
        NoRestart,
        &[
            d(UPDATE_AU, "NoAutoUpdate", 0),
            d(UPDATE_AU, "AUOptions", 2),
        ],
    ),
    def(
        "no_auto_reboot",
        Settings,
        NoRestart,
        &[d(UPDATE_AU, "NoAutoRebootWithLoggedOnUsers", 1)],
    ),
    // Boost: отдельные оптимизации
    def(
        "game_dvr",
        Boost,
        NoRestart,
        &[
            d(r"System\GameConfigStore", "GameDVR_Enabled", 0),
            d(GAME_DVR, "AppCaptureEnabled", 0),
            d(GAME_DVR, "HistoricalCaptureEnabled", 0).from(WIN10_1809),
        ],
    ),
    def(
        "game_dvr_policy",
        Boost,
        NoRestart,
        &[d(
            r"HKLM\SOFTWARE\Policies\Microsoft\Windows\GameDVR",
            "AllowGameDVR",
            0,
        )],
    ),
    def(
        "game_mode",
        Boost,
        NoRestart,
        &[
            d(r"Software\Microsoft\GameBar", "AutoGameModeEnabled", 1),
            d(r"Software\Microsoft\GameBar", "AllowAutoGameMode", 1),
        ],
    ),
    def(
        "power_throttling",
        Boost,
        NoRestart,
        &[d(
            r"HKLM\SYSTEM\CurrentControlSet\Control\Power\PowerThrottling",
            "PowerThrottlingOff",
            1,
        )],
    ),
    TweakDef {
        id: "ultimate",
        group: Boost,
        restart: NoRestart,
        dangerous: false,
        power_scheme: Some(ULTIMATE_SCHEME),
        entries: &[],
        policies: &[],
    },
    def(
        "hags",
        Boost,
        Reboot,
        &[d(
            r"HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            "HwSchMode",
            2,
        )
        .from(WIN10_2004)],
    ),
    TweakDef {
        id: "vbs_off",
        group: Boost,
        restart: Reboot,
        dangerous: true,
        power_scheme: None,
        entries: &[
            d(DEVICE_GUARD, "EnableVirtualizationBasedSecurity", 0),
            d(
                r"HKLM\SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
                "Enabled",
                0,
            ),
        ],
        policies: &[],
    },
];
