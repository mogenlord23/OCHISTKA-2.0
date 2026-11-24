use std::collections::HashSet;
use std::sync::atomic::AtomicBool;

use ochistka_core::fs::FsPort;
use ochistka_core::scan::{sweep_computer_scoped, ScanProgress, ScanReport, VirusScanPort};
use ochistka_core::system::CoreError;

pub fn scan_computer(
    fs: &dyn FsPort,
    scanner: &dyn VirusScanPort,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanReport, CoreError> {
    let (roots, mount_points) = scan_volumes()?;
    sweep_computer_scoped(
        fs,
        scanner,
        &roots,
        &|p| mount_points.contains(p),
        cancel,
        progress,
    )
}

#[cfg(windows)]
fn scan_volumes() -> Result<(Vec<String>, HashSet<String>), CoreError> {
    Ok((
        crate::windows::win32_ffi::fixed_drive_roots(),
        HashSet::new(),
    ))
}

#[cfg(target_os = "linux")]
fn scan_volumes() -> Result<(Vec<String>, HashSet<String>), CoreError> {
    let mounts = std::fs::read("/proc/self/mountinfo")
        .map_err(|err| CoreError::Probe(format!("список дисков недоступен: {err}")))?;
    Ok(linux_volumes(&mounts))
}

#[cfg(any(target_os = "linux", test))]
fn linux_volumes(mounts: &[u8]) -> (Vec<String>, HashSet<String>) {
    let mut roots = Vec::new();
    let mut points = HashSet::new();
    let mut overlapping = HashSet::new();
    let mut selected = HashSet::new();
    // Строки с не-UTF-8 путями всё равно отмечаются недоступными файловым адаптером.
    for line in mounts
        .split(|byte| *byte == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
    {
        let Some((options, filesystem)) = line.split_once(" - ") else {
            continue;
        };
        let mut fields = options.split_ascii_whitespace().skip(2);
        let (Some(id), Some(volume_root), Some(raw_path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let path = raw_path
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\");
        if !path.starts_with('/') {
            continue;
        }
        if !points.insert(path.clone()) {
            overlapping.insert(path.clone());
        }
        let mut fields = filesystem.split_ascii_whitespace();
        let (Some(kind), Some(source)) = (fields.next(), fields.next()) else {
            continue;
        };
        let local_disk = source.starts_with("/dev/")
            && matches!(
                kind,
                "ext2"
                    | "ext3"
                    | "ext4"
                    | "btrfs"
                    | "xfs"
                    | "f2fs"
                    | "bcachefs"
                    | "jfs"
                    | "nilfs2"
                    | "vfat"
                    | "exfat"
                    | "ntfs"
                    | "ntfs3"
                    | "fuseblk"
                    | "udf"
            );
        if (local_disk || kind == "zfs")
            && (volume_root == "/" || matches!(kind, "btrfs" | "zfs"))
            && selected.insert((id, volume_root))
        {
            roots.push(path);
        }
    }
    // Если таблица содержит несколько слоёв по одному пути, не угадываем доступный слой.
    roots.retain(|path| !overlapping.contains(path));
    roots.sort();
    (roots, points)
}

#[cfg(target_os = "macos")]
fn scan_volumes() -> Result<(Vec<String>, HashSet<String>), CoreError> {
    use std::mem::{size_of, MaybeUninit};
    use std::path::Path;

    let error = || CoreError::Probe("список локальных томов macOS недоступен".into());
    // SAFETY: null и длина 0 запрашивают только число записей.
    let count = unsafe { libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT) };
    let slots = usize::try_from(count).map_err(|_| error())? + 16;
    let bytes = i32::try_from(slots * size_of::<libc::statfs>()).map_err(|_| error())?;
    let mut mounts = vec![MaybeUninit::<libc::statfs>::uninit(); slots];
    // SAFETY: массив содержит slots записей, bytes передаёт его полный размер в байтах.
    let written = unsafe { libc::getfsstat(mounts.as_mut_ptr().cast(), bytes, libc::MNT_NOWAIT) };
    let written = usize::try_from(written).map_err(|_| error())?;
    if written >= slots {
        return Err(error());
    }

    let read_field = |field: &[libc::c_char]| {
        let bytes: Vec<u8> = field
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        String::from_utf8(bytes).ok()
    };
    let mut roots = Vec::new();
    let mut points = HashSet::new();
    for mount in mounts.into_iter().take(written) {
        // SAFETY: getfsstat успешно вернул число полностью записанных структур.
        let mount = unsafe { mount.assume_init() };
        let Some(path) = read_field(&mount.f_mntonname) else {
            continue;
        };
        if !path.starts_with('/') {
            continue;
        }
        points.insert(path.clone());
        let local = mount.f_flags & libc::MNT_LOCAL as u32 != 0;
        let disk_fs = read_field(&mount.f_fstypename).is_some_and(|kind| {
            matches!(
                kind.as_str(),
                "apfs" | "hfs" | "msdos" | "exfat" | "ntfs" | "udf"
            )
        });
        let data_volume = path == "/"
            || path == "/System/Volumes/Data"
            || Path::new(&path).starts_with("/Volumes");
        if local && disk_fs && data_volume {
            roots.push(path);
        }
    }
    roots.sort();
    roots.dedup();
    Ok((roots, points))
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn scan_volumes() -> Result<(Vec<String>, HashSet<String>), CoreError> {
    Err(CoreError::Unsupported {
        feature: "проверка компьютера".into(),
        os: std::env::consts::OS.into(),
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn open_regular(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "объект не является обычным файлом",
        ));
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_table_excludes_remote_pseudo_and_duplicate_volumes() {
        let mounts = b"24 12 8:1 / / rw - ext4 /dev/sda1 rw\n\
            25 24 0:44 / /proc rw - proc proc rw\n\
            26 24 0:45 / /mnt/remote rw - nfs server:/exports rw\n\
            27 24 8:2 / /media/My\\040Disk rw - ext4 /dev/sdb1 rw\n\
            28 24 8:2 / /mnt/alias rw - ext4 /dev/sdb1 rw\n\
            29 24 8:3 / /mnt/hidden rw - ext4 /dev/sdc1 rw\n\
            30 24 0:46 / /mnt/hidden rw - nfs server:/overmount rw\n";
        let (roots, points) = linux_volumes(mounts);
        assert_eq!(roots, ["/", "/media/My Disk"]);
        assert!(points.contains("/proc"));
        assert!(points.contains("/mnt/remote"));
        assert!(points.contains("/mnt/alias"));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn host_boot_volume_is_discoverable() {
        let (roots, mount_points) = scan_volumes().unwrap();
        assert!(roots.iter().any(|path| path == "/"));
        assert!(mount_points.contains("/"));
    }
}
