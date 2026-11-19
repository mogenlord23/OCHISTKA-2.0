//! Драйверы: каталог по производителям, разрешённые адреса загрузки, разбор проверки подписи
//! и порядок установки. Без кода ОС: загрузку, проверку подписи и запуск выполняет порт хоста,
//! адаптер платформы (см. ADR 0011). Установка идёт только через официальные установщики
//! производителей, которые сами запрашивают права администратора.

use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;
use url::Url;

/// Верхняя граница размера установщика, байт.
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

/// Домены, с которых разрешено качать установщики. Поддомены допускаются, остальное — нет.
pub const ALLOWED_DOMAINS: &[&str] = &["nvidia.com", "intel.com", "amd.com", "realtek.com"];

/// Единственная системная страница, которую приложение открывает не по https.
pub const WINDOWS_UPDATE_URI: &str = "ms-settings:windowsupdate-optionalupdates";

/// Сколько переадресаций допускаем при загрузке.
pub const MAX_REDIRECTS: usize = 5;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DriverError {
    #[error("Неизвестный драйвер.")]
    UnknownItem,
    #[error("Для этого пункта нет прямого установщика: откройте официальный сайт.")]
    NoInstaller,
    #[error("Адрес отклонён: {0}")]
    UrlRejected(String),
    #[error("Установщик превышает допустимый размер.")]
    TooLarge,
    #[error("Не удалось скачать установщик: {0}")]
    Download(String),
    #[error("Цифровая подпись не подтверждена: {0}")]
    SignatureInvalid(String),
    #[error("Подпись принадлежит другому издателю: {actual}")]
    SignerMismatch { actual: String },
    #[error("Не удалось запустить установщик: {0}")]
    Launch(String),
    #[error("Эта функция недоступна в текущей ОС ({0}).")]
    Unsupported(String),
}

/// Прямой установщик производителя.
#[derive(Debug, Clone, Copy)]
pub struct InstallerSource {
    pub url: &'static str,
    /// Имя файла в папке загрузок; только буквы, цифры, точки, дефисы.
    pub file_name: &'static str,
    /// Допустимые имена издателя в сертификате подписи.
    pub signers: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct CatalogEntry {
    pub id: &'static str,
    /// Идентификатор плитки-категории: `nvidia`, `amd`, `intel`, `audio` («Звук и связь»), `usb`, `oem`.
    pub vendor: &'static str,
    /// Производитель драйвера (показывается внутри плитки): «Realtek», «Intel».
    pub maker: &'static str,
    /// Тип устройства: «Звук», «Wi-Fi», «Bluetooth».
    pub device: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Официальная страница производителя (для «Открыть сайт»).
    pub page_url: &'static str,
    pub installer: Option<InstallerSource>,
    /// Драйвер может поставить Windows Update.
    pub windows_update: bool,
}

pub const VENDOR_IDS: &[&str] = &["nvidia", "amd", "intel", "audio", "usb", "oem"];

const INTEL: &[&str] = &["Intel Corporation"];

const fn entry(
    id: &'static str,
    vendor: &'static str,
    maker: &'static str,
    device: &'static str,
    title: &'static str,
    description: &'static str,
    page_url: &'static str,
    installer: Option<InstallerSource>,
    windows_update: bool,
) -> CatalogEntry {
    CatalogEntry {
        id,
        vendor,
        maker,
        device,
        title,
        description,
        page_url,
        installer,
        windows_update,
    }
}

/// Каталог собран по каталогу прежнего приложения 0.5.5: те же производители и страницы.
/// AMD: прямую ссылку 0.5.5 сервер отдаёт только со страницы amd.com (требует заголовок
/// Referer), поэтому для AMD кнопка открывает сайт.
pub static CATALOG: &[CatalogEntry] = &[
    entry(
        "nvidia-app",
        "nvidia",
        "NVIDIA",
        "Видеокарта",
        "NVIDIA App · драйверы и библиотека игр",
        "Официальное приложение NVIDIA: драйверы Game Ready и Studio, настройки игр и запись игрового процесса.",
        "https://www.nvidia.com/en-us/software/nvidia-app/",
        Some(InstallerSource {
            url: "https://us.download.nvidia.com/nvapp/client/11.0.9.565/NVIDIA_app_v11.0.9.565.exe",
            file_name: "NVIDIA-app.exe",
            signers: &["NVIDIA Corporation"],
        }),
        false,
    ),
    entry(
        "nvidia-geforce",
        "nvidia",
        "NVIDIA",
        "Видеокарта",
        "GeForce Game Ready / Studio",
        "Выберите точную модель видеокарты и ветку драйвера на странице NVIDIA.",
        "https://www.nvidia.com/en-us/geforce/drivers/",
        None,
        true,
    ),
    entry(
        "nvidia-physx",
        "nvidia",
        "NVIDIA",
        "Физика в играх",
        "PhysX System Software",
        "Дополнительная библиотека для старых игр, которым нужен NVIDIA PhysX.",
        "https://www.nvidia.com/en-us/drivers/physx/physx-9-21-0713-driver/",
        None,
        false,
    ),
    entry(
        "nvidia-cuda",
        "nvidia",
        "NVIDIA",
        "Вычисления",
        "CUDA Toolkit",
        "Для приложений и игр, которым нужны вычисления на видеокарте. Сверьте версию с требованиями программы.",
        "https://developer.nvidia.com/cuda-downloads",
        None,
        false,
    ),
    entry(
        "amd-software",
        "amd",
        "AMD",
        "Видеокарта",
        "AMD Software · Adrenalin",
        "Официальный установщик драйвера Radeon. Версию и совместимость проверяет сам производитель.",
        "https://www.amd.com/en/support/download/drivers.html",
        None,
        true,
    ),
    entry(
        "amd-detect",
        "amd",
        "AMD",
        "Видеокарта",
        "AMD · автоопределение",
        "Официальное автоопределение Radeon и чипсета Ryzen на сайте AMD.",
        "https://www.amd.com/en/support/download/drivers.html",
        None,
        false,
    ),
    entry(
        "amd-chipset",
        "amd",
        "AMD",
        "Чипсет",
        "AMD · чипсет",
        "Выберите сокет и модель платы на сайте AMD. Драйвер процессора отдельно обычно не требуется.",
        "https://www.amd.com/en/support/download/drivers.html",
        None,
        true,
    ),
    entry(
        "intel-dsa",
        "intel",
        "Intel",
        "Все устройства Intel",
        "Intel · автоопределение",
        "Официальный помощник Intel подбирает поддерживаемые драйверы для устройств Intel.",
        "https://www.intel.com/content/www/us/en/support/detect.html",
        Some(InstallerSource {
            url: "https://dsadata.intel.com/installer",
            file_name: "Intel-Driver-and-Support-Assistant.exe",
            signers: INTEL,
        }),
        false,
    ),
    entry(
        "intel-chipset",
        "intel",
        "Intel",
        "Чипсет",
        "Intel · чипсет",
        "Официальные пакеты чипсета. Совместимость зависит от модели платы и поколения процессора.",
        "https://www.intel.com/content/www/us/en/download/19347/chipset-inf-utility.html",
        None,
        true,
    ),
    entry(
        "intel-me",
        "intel",
        "Intel",
        "Management Engine",
        "Intel Management Engine",
        "Пакет берётся по точной модели системы или материнской платы.",
        "https://www.intel.com/content/www/us/en/download-center/home.html",
        None,
        true,
    ),
    entry(
        "intel-wifi",
        "audio",
        "Intel",
        "Wi-Fi",
        "Intel · Wi-Fi",
        "Пакет для поддерживаемых беспроводных адаптеров Intel под Windows 10 и 11.",
        "https://www.intel.com/content/www/us/en/download/19351/intel-wireless-wi-fi-drivers-for-windows-10-and-windows-11.html",
        Some(InstallerSource {
            url: "https://downloadmirror.intel.com/929786/WiFi-24.80.0-Driver64-Win10-Win11.exe",
            file_name: "Intel-WiFi-24.80.0.exe",
            signers: INTEL,
        }),
        true,
    ),
    entry(
        "intel-bluetooth",
        "audio",
        "Intel",
        "Bluetooth",
        "Intel · Bluetooth",
        "Выберите пакет под свой адаптер и версию Windows.",
        "https://www.intel.com/content/www/us/en/download/18649/intel-wireless-bluetooth-drivers-for-windows-10-and-windows-11.html",
        None,
        true,
    ),
    entry(
        "realtek-audio",
        "audio",
        "Realtek",
        "Звук",
        "Realtek · звук",
        "Каталог звуковых драйверов. Часто надёжнее взять пакет у производителя платы или ноутбука.",
        "https://www.realtek.com/Download/List?cate_id=593&menu_id=369",
        None,
        true,
    ),
    entry(
        "realtek-network",
        "audio",
        "Realtek",
        "Сеть: Ethernet и Wi-Fi",
        "Realtek · Ethernet и Wi-Fi",
        "Каталог сетевых контроллеров Realtek. Подбирайте по модели устройства.",
        "https://www.realtek.com/Download/List?cate_id=584&menu_id=297",
        None,
        true,
    ),
    entry(
        "audio-other",
        "audio",
        "Другие производители",
        "Звук",
        "Другие звуковые устройства",
        "Звук от Creative, Conexant, VIA и других: сначала Windows Update, затем пакет производителя платы или ноутбука.",
        "https://support.microsoft.com/en-us/windows/hardware/drivers/update-drivers-through-device-manager-in-windows",
        None,
        true,
    ),
    entry(
        "wifi-other",
        "audio",
        "Другие производители",
        "Wi-Fi",
        "Wi-Fi: другие адаптеры",
        "Адаптеры Qualcomm, MediaTek, Broadcom и другие: драйвер ставит Windows Update или производитель ноутбука.",
        "https://support.microsoft.com/en-us/windows/hardware/drivers/update-drivers-through-device-manager-in-windows",
        None,
        true,
    ),
    entry(
        "bluetooth-other",
        "audio",
        "Другие производители",
        "Bluetooth",
        "Bluetooth: другие адаптеры",
        "Адаптеры не от Intel: драйвер ставит Windows Update или производитель ноутбука или платы.",
        "https://support.microsoft.com/en-us/windows/hardware/drivers/update-drivers-through-device-manager-in-windows",
        None,
        true,
    ),
    entry(
        "usb-controllers",
        "usb",
        "Microsoft",
        "USB и контроллеры",
        "USB / контроллеры",
        "Стандартные USB-драйверы входят в Windows. Для контроллера платы возьмите пакет производителя.",
        "https://support.microsoft.com/en-us/windows/hardware/drivers/update-drivers-through-device-manager-in-windows",
        None,
        true,
    ),
    entry(
        "logitech",
        "usb",
        "Logitech",
        "Периферия",
        "Logitech · периферия",
        "Пакет для мышей, клавиатур, камер и игровых контроллеров Logitech.",
        "https://www.logitech.com/en-us/software.html",
        None,
        false,
    ),
    entry(
        "asus",
        "oem",
        "ASUS",
        "Ноутбук или плата",
        "ASUS · ноутбук или плата",
        "Поиск по точной модели: Wi-Fi, USB, звук и чипсет.",
        "https://www.asus.com/support/download-center/",
        None,
        true,
    ),
    entry(
        "msi",
        "oem",
        "MSI",
        "Ноутбук или плата",
        "MSI · ноутбук или плата",
        "Официальные пакеты для конкретной модели компьютера или материнской платы.",
        "https://www.msi.com/support/download",
        None,
        true,
    ),
    entry(
        "gigabyte",
        "oem",
        "GIGABYTE",
        "Плата",
        "GIGABYTE · плата",
        "Выберите модель и ревизию платы перед загрузкой драйвера.",
        "https://www.gigabyte.com/Support/Consumer/Download",
        None,
        true,
    ),
    entry(
        "lenovo",
        "oem",
        "Lenovo",
        "Компьютер",
        "Lenovo · компьютер",
        "Подбор пакетов по модели или серийному номеру на сайте производителя.",
        "https://pcsupport.lenovo.com/",
        None,
        true,
    ),
    entry(
        "dell",
        "oem",
        "Dell",
        "Компьютер",
        "Dell · компьютер",
        "Официальные драйверы по модели или Service Tag.",
        "https://www.dell.com/support/home/drivers",
        None,
        true,
    ),
    entry(
        "hp",
        "oem",
        "HP",
        "Компьютер",
        "HP · компьютер",
        "Официальные пакеты для точной модели компьютера.",
        "https://support.hp.com/drivers",
        None,
        true,
    ),
    entry(
        "acer",
        "oem",
        "Acer",
        "Компьютер",
        "Acer · компьютер",
        "Официальная загрузка по модели или SNID.",
        "https://www.acer.com/us-en/support/drivers-and-manuals",
        None,
        true,
    ),
    entry(
        "surface",
        "oem",
        "Microsoft",
        "Surface",
        "Microsoft Surface",
        "Драйверы и прошивка для точной модели Surface.",
        "https://www.microsoft.com/en-us/download/details.aspx?id=100429",
        None,
        true,
    ),
];

pub fn find(id: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|item| item.id == id)
}

/// Пункты одного производителя; пустое значение — весь каталог.
pub fn entries_for_vendor(vendor: Option<&str>) -> Vec<&'static CatalogEntry> {
    CATALOG
        .iter()
        .filter(|item| vendor.is_none_or(|v| item.vendor == v))
        .collect()
}

// --- Адреса ---------------------------------------------------------------------------------

fn host_allowed(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    ALLOWED_DOMAINS
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

/// Адрес загрузки: только https, без логина и нестандартного порта, домен из белого списка.
pub fn validate_download_url(raw: &str) -> Result<Url, DriverError> {
    let reject = |why: &str| DriverError::UrlRejected(why.to_string());
    let url = Url::parse(raw).map_err(|_| reject("некорректный адрес"))?;
    if url.scheme() != "https" {
        return Err(reject("разрешён только https"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(reject("логин в адресе не допускается"));
    }
    if url.port().is_some() {
        return Err(reject("нестандартный порт не допускается"));
    }
    match url.host_str() {
        Some(host) if host_allowed(host) => Ok(url),
        Some(host) => Err(reject(&format!("домен {host} не в списке производителей"))),
        None => Err(reject("в адресе нет домена")),
    }
}

/// Переадресация проверяется так же строго, как исходный адрес.
pub fn validate_redirect(target: &str) -> Result<Url, DriverError> {
    validate_download_url(target)
}

/// Страница для открытия в браузере: только https либо страница обновлений Windows.
pub fn validate_page_url(raw: &str) -> Result<(), DriverError> {
    if raw == WINDOWS_UPDATE_URI {
        return Ok(());
    }
    let url = Url::parse(raw).map_err(|_| DriverError::UrlRejected("некорректный адрес".into()))?;
    if url.scheme() == "https" && url.host_str().is_some() && url.username().is_empty() {
        Ok(())
    } else {
        Err(DriverError::UrlRejected("разрешён только https".into()))
    }
}

// --- Подпись --------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInfo {
    pub status: String,
    pub subject: String,
}

/// Скрипт проверки. Путь передаётся переменной окружения, а не в тексте скрипта.
pub const SIGNATURE_SCRIPT: &str =
    "$s = Get-AuthenticodeSignature -LiteralPath $env:OCHISTKA_CHECK_FILE; \
    'Status=' + $s.Status; 'Subject=' + $s.SignerCertificate.Subject";

/// Разбирает вывод скрипта (`Status=...`, `Subject=...`). Лишние строки игнорируются.
pub fn parse_signature_output(text: &str) -> Result<SignatureInfo, DriverError> {
    let mut status = None;
    let mut subject = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Status=") {
            status = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("Subject=") {
            subject = Some(value.trim().to_string());
        }
    }
    let status = status
        .filter(|s| !s.is_empty())
        .ok_or_else(|| DriverError::SignatureInvalid("нет ответа проверки".into()))?;
    Ok(SignatureInfo {
        status,
        subject: subject.unwrap_or_default(),
    })
}

/// Разбирает DN сертификата («CN=..., O="A, Inc.", C=US») в пары атрибут-значение.
pub fn parse_subject(subject: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut in_key = true;
    let mut quoted = false;
    let mut chars = subject.chars().peekable();
    let flush = |key: &mut String, value: &mut String, out: &mut Vec<(String, String)>| {
        if !key.trim().is_empty() {
            out.push((key.trim().to_ascii_uppercase(), value.trim().to_string()));
        }
        key.clear();
        value.clear();
    };
    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_key => quoted = !quoted,
            '\\' if !in_key => {
                if let Some(next) = chars.next() {
                    value.push(next);
                }
            }
            '=' if in_key => in_key = false,
            ',' if !quoted && !in_key => {
                flush(&mut key, &mut value, &mut result);
                in_key = true;
            }
            _ if in_key => key.push(ch),
            _ => value.push(ch),
        }
    }
    flush(&mut key, &mut value, &mut result);
    result
}

/// Издатель подходит, если CN или O сертификата равен одному из допустимых имён.
pub fn signer_matches(subject: &str, allowed: &[&str]) -> bool {
    parse_subject(subject).iter().any(|(key, value)| {
        (key == "CN" || key == "O") && allowed.iter().any(|a| a.eq_ignore_ascii_case(value))
    })
}

/// Подпись должна быть действительной, а издатель — совпадать с ожидаемым.
pub fn check_signature(info: &SignatureInfo, allowed: &[&str]) -> Result<(), DriverError> {
    if info.status != "Valid" {
        return Err(DriverError::SignatureInvalid(format!(
            "статус {}",
            info.status
        )));
    }
    if signer_matches(&info.subject, allowed) {
        Ok(())
    } else {
        let actual = parse_subject(&info.subject)
            .into_iter()
            .find(|(k, _)| k == "O" || k == "CN")
            .map(|(_, v)| v)
            .unwrap_or_else(|| "неизвестно".into());
        Err(DriverError::SignerMismatch { actual })
    }
}

// --- Установка ------------------------------------------------------------------------------

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Downloading,
    Verifying,
    Launching,
    Done,
    Error,
}

/// Состояние установки для интерфейса.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub id: String,
    pub stage: Stage,
    /// Для `Downloading`: процент, если известен размер.
    pub percent: Option<u8>,
    /// Для `Error`: причина по-русски.
    pub message: Option<String>,
}

impl InstallProgress {
    fn new(id: &str, stage: Stage, percent: Option<u8>, message: Option<String>) -> Self {
        Self {
            id: id.to_string(),
            stage,
            percent,
            message,
        }
    }
}

pub fn percent(done: u64, total: Option<u64>) -> Option<u8> {
    let total = total.filter(|t| *t > 0)?;
    Some((done.min(total) * 100 / total) as u8)
}

/// Всё системное, что нужно установке. Адаптер платформы на Windows, заглушка на других ОС.
pub trait DriverHost: Send + Sync {
    /// Папка загрузок (`%LOCALAPPDATA%\OCHISTKA\Downloads`), создаётся при необходимости.
    fn downloads_dir(&self) -> Result<PathBuf, DriverError>;
    /// Скачивает по https в файл. Каждую переадресацию проверяет `validate_redirect`.
    /// `on_progress(получено, всего)`.
    fn download(
        &self,
        url: &Url,
        dest: &Path,
        max_bytes: u64,
        on_progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), DriverError>;
    /// Вывод `SIGNATURE_SCRIPT` для файла.
    fn signature_output(&self, file: &Path) -> Result<String, DriverError>;
    fn rename(&self, from: &Path, to: &Path) -> Result<(), DriverError>;
    fn remove(&self, file: &Path);
    /// Запуск установщика; он сам просит права администратора.
    fn launch(&self, file: &Path) -> Result<(), DriverError>;
    /// Открывает адрес в браузере по умолчанию (или страницу обновлений Windows).
    fn open_url(&self, url: &str) -> Result<(), DriverError>;
}

/// Скачать, проверить подпись, запустить. Каждый этап сообщается через `report`.
/// При любом сбое непроверенный файл удаляется.
pub fn install(
    host: &dyn DriverHost,
    id: &str,
    report: &mut dyn FnMut(InstallProgress),
) -> Result<(), DriverError> {
    let result = install_inner(host, id, report);
    match &result {
        Ok(()) => report(InstallProgress::new(id, Stage::Done, None, None)),
        Err(err) => report(InstallProgress::new(
            id,
            Stage::Error,
            None,
            Some(err.to_string()),
        )),
    }
    result
}

fn install_inner(
    host: &dyn DriverHost,
    id: &str,
    report: &mut dyn FnMut(InstallProgress),
) -> Result<(), DriverError> {
    let item = find(id).ok_or(DriverError::UnknownItem)?;
    let source = item.installer.ok_or(DriverError::NoInstaller)?;
    let url = validate_download_url(source.url)?;
    let dir = host.downloads_dir()?;
    let part = dir.join(format!("{}.part", source.file_name));
    let target = dir.join(source.file_name);

    report(InstallProgress::new(id, Stage::Downloading, Some(0), None));
    let mut last = 0u8;
    let downloaded = host.download(&url, &part, MAX_DOWNLOAD_BYTES, &mut |done, total| {
        if let Some(p) = percent(done, total) {
            if p != last {
                last = p;
                report(InstallProgress::new(id, Stage::Downloading, Some(p), None));
            }
        }
    });
    if let Err(err) = downloaded {
        host.remove(&part);
        return Err(err);
    }

    report(InstallProgress::new(id, Stage::Verifying, None, None));
    let checked = host
        .signature_output(&part)
        .and_then(|text| parse_signature_output(&text))
        .and_then(|info| check_signature(&info, source.signers));
    if let Err(err) = checked {
        host.remove(&part);
        return Err(err);
    }
    host.remove(&target);
    host.rename(&part, &target)?;

    report(InstallProgress::new(id, Stage::Launching, None, None));
    host.launch(&target)
}

/// Открывает официальную страницу пункта в браузере.
pub fn open_page(host: &dyn DriverHost, id: &str) -> Result<(), DriverError> {
    let item = find(id).ok_or(DriverError::UnknownItem)?;
    validate_page_url(item.page_url)?;
    host.open_url(item.page_url)
}

/// Открывает «Необязательные обновления» Windows.
pub fn open_windows_update(host: &dyn DriverHost) -> Result<(), DriverError> {
    host.open_url(WINDOWS_UPDATE_URI)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Mutex;

    #[test]
    fn catalog_ids_unique_and_vendors_known() {
        let mut seen = HashSet::new();
        for item in CATALOG {
            assert!(seen.insert(item.id), "дубль {}", item.id);
            assert!(VENDOR_IDS.contains(&item.vendor), "{}", item.id);
            validate_page_url(item.page_url).unwrap();
        }
        for vendor in VENDOR_IDS {
            assert!(!entries_for_vendor(Some(vendor)).is_empty(), "{vendor}");
        }
    }

    #[test]
    fn sound_and_connectivity_category_replaces_the_separate_realtek_tile() {
        assert!(!VENDOR_IDS.contains(&"realtek") && !VENDOR_IDS.contains(&"bluetooth"));
        let audio: Vec<&str> = entries_for_vendor(Some("audio"))
            .iter()
            .map(|i| i.id)
            .collect();
        for id in [
            "realtek-audio",
            "realtek-network",
            "audio-other",
            "intel-wifi",
            "wifi-other",
            "intel-bluetooth",
            "bluetooth-other",
        ] {
            assert!(audio.contains(&id), "{id}");
        }
        for item in CATALOG {
            assert!(
                !item.maker.is_empty() && !item.device.is_empty(),
                "{}",
                item.id
            );
        }
    }

    #[test]
    fn filtering_by_vendor_returns_only_that_vendor() {
        let nvidia = entries_for_vendor(Some("nvidia"));
        assert!(nvidia.len() >= 3);
        assert!(nvidia.iter().all(|i| i.vendor == "nvidia"));
        assert_eq!(entries_for_vendor(None).len(), CATALOG.len());
        assert!(entries_for_vendor(Some("nope")).is_empty());
    }

    #[test]
    fn every_installer_passes_the_allow_list() {
        for item in CATALOG {
            if let Some(src) = item.installer {
                validate_download_url(src.url).unwrap();
                assert!(!src.signers.is_empty());
                assert!(src
                    .file_name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'));
            }
        }
    }

    #[test]
    fn rejects_http_other_domains_and_tricks() {
        for bad in [
            "http://us.download.nvidia.com/a.exe",
            "https://evil.com/a.exe",
            "https://nvidia.com.evil.com/a.exe",
            "https://evilnvidia.com/a.exe",
            "https://nvidia.com@evil.com/a.exe",
            "https://user:pw@nvidia.com/a.exe",
            "https://nvidia.com:8443/a.exe",
            "ftp://nvidia.com/a.exe",
            "file:///C:/a.exe",
            "https://127.0.0.1/a.exe",
            "not a url",
        ] {
            assert!(validate_download_url(bad).is_err(), "{bad}");
        }
        for good in [
            "https://nvidia.com/a.exe",
            "https://us.download.nvidia.com/a.exe",
            "https://DOWNLOADMIRROR.INTEL.COM/a.exe",
        ] {
            assert!(validate_download_url(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn redirects_are_checked_like_the_first_url() {
        assert!(validate_redirect("https://downloadmirror.intel.com/x.exe").is_ok());
        assert!(validate_redirect("https://evil.com/x.exe").is_err());
        assert!(validate_redirect("http://downloadmirror.intel.com/x.exe").is_err());
    }

    /// Адреса, которые при проверке (docs/status/drivers-links.md) вели в ошибку 404.
    #[test]
    fn catalog_has_no_known_dead_links() {
        for item in CATALOG {
            for dead in [
                "https://www.nvidia.com/en-us/drivers/physx/",
                "https://support.microsoft.com/windows/get-updates-for-windows-drivers",
                "download/18649/intel-wireless-bluetooth-for-windows-10-and-windows-11.html",
            ] {
                assert_ne!(item.page_url, dead, "{}", item.id);
            }
        }
    }

    #[test]
    fn page_urls_https_or_windows_update_only() {
        assert!(validate_page_url("https://www.asus.com/support/").is_ok());
        assert!(validate_page_url(WINDOWS_UPDATE_URI).is_ok());
        assert!(validate_page_url("http://www.asus.com").is_err());
        assert!(validate_page_url("ms-settings:other").is_err());
        assert!(validate_page_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn parses_signature_output() {
        let info = parse_signature_output(
            "Status=Valid\r\nSubject=CN=NVIDIA Corporation, O=NVIDIA Corporation, L=Santa Clara, C=US\r\n",
        )
        .unwrap();
        assert_eq!(info.status, "Valid");
        assert!(signer_matches(&info.subject, &["NVIDIA Corporation"]));
        assert!(parse_signature_output("").is_err());
        assert!(parse_signature_output("garbage").is_err());
        let unsigned = parse_signature_output("Status=NotSigned\nSubject=").unwrap();
        assert_eq!(unsigned.subject, "");
    }

    #[test]
    fn subject_with_quoted_comma() {
        let attrs = parse_subject(
            r#"CN="Advanced Micro Devices, Inc.", O="Advanced Micro Devices, Inc.", C=US"#,
        );
        assert_eq!(
            attrs[0],
            ("CN".into(), "Advanced Micro Devices, Inc.".into())
        );
        assert!(signer_matches(
            r#"CN="Advanced Micro Devices, Inc.", C=US"#,
            &["Advanced Micro Devices, Inc."]
        ));
    }

    #[test]
    fn signer_must_equal_not_contain() {
        assert!(!signer_matches("CN=Not Intel Corporation Ltd", INTEL));
        assert!(!signer_matches("CN=Evil, OU=Intel Corporation", INTEL));
        assert!(signer_matches("CN=intel corporation", INTEL));
    }

    #[test]
    fn check_signature_cases() {
        let ok = SignatureInfo {
            status: "Valid".into(),
            subject: "CN=Intel Corporation, O=Intel Corporation".into(),
        };
        assert!(check_signature(&ok, INTEL).is_ok());
        let bad = SignatureInfo {
            status: "HashMismatch".into(),
            ..ok.clone()
        };
        assert!(matches!(
            check_signature(&bad, INTEL),
            Err(DriverError::SignatureInvalid(_))
        ));
        let other = SignatureInfo {
            status: "Valid".into(),
            subject: "CN=Somebody Else".into(),
        };
        assert_eq!(
            check_signature(&other, INTEL),
            Err(DriverError::SignerMismatch {
                actual: "Somebody Else".into()
            })
        );
    }

    #[test]
    fn percent_math() {
        assert_eq!(percent(50, Some(200)), Some(25));
        assert_eq!(percent(500, Some(200)), Some(100));
        assert_eq!(percent(5, None), None);
        assert_eq!(percent(5, Some(0)), None);
    }

    #[derive(Default)]
    struct Fake {
        sig: String,
        fail_download: bool,
        fail_launch: bool,
        calls: Mutex<Vec<String>>,
    }
    impl DriverHost for Fake {
        fn downloads_dir(&self) -> Result<PathBuf, DriverError> {
            Ok(PathBuf::from("dl"))
        }
        fn download(
            &self,
            url: &Url,
            _dest: &Path,
            _max: u64,
            on_progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<(), DriverError> {
            self.calls.lock().unwrap().push(format!("download {url}"));
            if self.fail_download {
                return Err(DriverError::Download("нет сети".into()));
            }
            on_progress(50, Some(100));
            on_progress(100, Some(100));
            Ok(())
        }
        fn signature_output(&self, _file: &Path) -> Result<String, DriverError> {
            Ok(self.sig.clone())
        }
        fn rename(&self, _f: &Path, _t: &Path) -> Result<(), DriverError> {
            self.calls.lock().unwrap().push("rename".into());
            Ok(())
        }
        fn remove(&self, file: &Path) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("remove {}", file.display()));
        }
        fn launch(&self, _file: &Path) -> Result<(), DriverError> {
            self.calls.lock().unwrap().push("launch".into());
            if self.fail_launch {
                Err(DriverError::Launch("отказ".into()))
            } else {
                Ok(())
            }
        }
        fn open_url(&self, url: &str) -> Result<(), DriverError> {
            self.calls.lock().unwrap().push(format!("open {url}"));
            Ok(())
        }
    }

    fn run(host: &Fake, id: &str) -> (Result<(), DriverError>, Vec<InstallProgress>) {
        let mut events = Vec::new();
        let result = install(host, id, &mut |p| events.push(p));
        (result, events)
    }

    #[test]
    fn install_happy_path_reports_every_stage() {
        let host = Fake {
            sig: "Status=Valid\nSubject=CN=NVIDIA Corporation".into(),
            ..Default::default()
        };
        let (result, events) = run(&host, "nvidia-app");
        assert!(result.is_ok());
        let stages: Vec<Stage> = events.iter().map(|e| e.stage).collect();
        assert_eq!(stages.first(), Some(&Stage::Downloading));
        assert!(events.iter().any(|e| e.percent == Some(50)));
        assert!(stages.contains(&Stage::Verifying));
        assert!(stages.contains(&Stage::Launching));
        assert_eq!(stages.last(), Some(&Stage::Done));
        assert!(host.calls.lock().unwrap().iter().any(|c| c == "launch"));
    }

    #[test]
    fn wrong_signer_removes_file_and_never_launches() {
        let host = Fake {
            sig: "Status=Valid\nSubject=CN=Somebody Else".into(),
            ..Default::default()
        };
        let (result, events) = run(&host, "nvidia-app");
        assert!(matches!(result, Err(DriverError::SignerMismatch { .. })));
        let calls = host.calls.lock().unwrap();
        assert!(calls.iter().any(|c| c.starts_with("remove")));
        assert!(!calls.iter().any(|c| c == "launch"));
        let last = events.last().unwrap();
        assert_eq!(last.stage, Stage::Error);
        assert!(last.message.as_ref().unwrap().contains("Somebody Else"));
    }

    #[test]
    fn download_failure_is_an_error_event() {
        let host = Fake {
            fail_download: true,
            ..Default::default()
        };
        let (result, events) = run(&host, "intel-dsa");
        assert!(matches!(result, Err(DriverError::Download(_))));
        assert_eq!(events.last().unwrap().stage, Stage::Error);
    }

    #[test]
    fn items_without_installer_or_unknown_are_refused() {
        let host = Fake::default();
        assert_eq!(run(&host, "asus").0, Err(DriverError::NoInstaller));
        assert_eq!(run(&host, "nope").0, Err(DriverError::UnknownItem));
        assert!(host.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn open_page_and_windows_update() {
        let host = Fake::default();
        open_page(&host, "asus").unwrap();
        open_windows_update(&host).unwrap();
        let calls = host.calls.lock().unwrap();
        assert_eq!(
            calls[0],
            "open https://www.asus.com/support/download-center/"
        );
        assert_eq!(calls[1], format!("open {WINDOWS_UPDATE_URI}"));
        assert_eq!(open_page(&host, "zzz"), Err(DriverError::UnknownItem));
    }
}
