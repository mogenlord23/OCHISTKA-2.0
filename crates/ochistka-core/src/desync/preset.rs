//! Exact-order reader for authentic Zapret 2 / NetZapret presets.
//!
//! Offline-only: parsing never runs a driver, changes network settings or
//! authorizes a profile. Unknown winws2 switches remain intact.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZapretPreset {
    pub name: String,
    pub builtin_version: Option<String>,
    pub global_arguments: Vec<String>,
    pub sections: Vec<ZapretSection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZapretSection {
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PresetError {
    #[error("В пресете отсутствует название")]
    MissingName,
    #[error("Некорректный аргумент пресета в строке {0}")]
    InvalidArgument(usize),
    #[error("Пустая секция пресета в строке {0}")]
    EmptySection(usize),
    #[error("Недопустимая ссылка на ресурс пресета: {0}")]
    UnsupportedReference(String),
    #[error("Небезопасный путь ресурса: {0}")]
    UnsafeResource(String),
    #[error("Нет каталога ресурсов: {0}")]
    MissingRoot(String),
    #[error("Нет ресурса пресета: {0}")]
    MissingResource(String),
}

impl ZapretPreset {
    /// Keep all options and their order. Like upstream NetZapret, everything
    /// preceding the *first* --new belongs to the global argument region.
    pub fn parse(input: &str) -> Result<Self, PresetError> {
        let mut name = None;
        let mut builtin_version = None;
        let mut global_arguments = Vec::new();
        let mut sections = Vec::new();
        let mut current = Vec::new();
        let mut seen_separator = false;

        for (index, raw) in input.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(comment) = line.strip_prefix('#') {
                let comment = comment.trim();
                if let Some(value) = comment.strip_prefix("Preset:") {
                    if name.is_none() && !value.trim().is_empty() {
                        name = Some(value.trim().to_owned());
                    }
                } else if let Some(value) = comment.strip_prefix("BuiltinVersion:") {
                    if builtin_version.is_none() && !value.trim().is_empty() {
                        builtin_version = Some(value.trim().to_owned());
                    }
                }
                continue;
            }
            for argument in split_arguments(line) {
                if argument == "--new" {
                    if current.is_empty() {
                        return Err(PresetError::EmptySection(index + 1));
                    }
                    if seen_separator {
                        sections.push(ZapretSection {
                            arguments: std::mem::take(&mut current),
                        });
                    } else {
                        global_arguments = std::mem::take(&mut current);
                        seen_separator = true;
                    }
                } else {
                    if !argument.starts_with("--")
                        || argument.chars().any(char::is_control)
                    {
                        return Err(PresetError::InvalidArgument(index + 1));
                    }
                    current.push(argument.to_owned());
                }
            }
        }

        if !current.is_empty() {
            if seen_separator {
                sections.push(ZapretSection { arguments: current });
            } else {
                global_arguments = current;
            }
        } else if seen_separator {
            return Err(PresetError::EmptySection(input.lines().count()));
        }
        if global_arguments.is_empty() {
            return Err(PresetError::InvalidArgument(1));
        }
        Ok(Self {
            name: name.ok_or(PresetError::MissingName)?,
            builtin_version,
            global_arguments,
            sections,
        })
    }

    /// Returns the entire preset, not a fabricated combination of recipes.
    pub fn command_arguments(&self) -> Vec<String> {
        let mut result = self.global_arguments.clone();
        for section in &self.sections {
            result.push("--new".to_owned());
            result.extend(section.arguments.iter().cloned());
        }
        result
    }

    /// Discover every referenced file without fetching, reading or trusting it.
    pub fn required_resources(&self) -> Result<BTreeSet<String>, PresetError> {
        let mut resources = BTreeSet::new();
        for argument in self.command_arguments() {
            for part in argument.split(|c: char| c == '=' || c == ':' || c == ',' || c.is_whitespace()) {
                if part.is_empty() {
                    continue;
                }
                let value = part.strip_prefix('@').unwrap_or(part);
                let recognized = ["lua/", "lists/", "bin/", "windivert.filter/"]
                    .iter()
                    .any(|prefix| value.starts_with(prefix));
                if part.starts_with('@') && !recognized {
                    return Err(PresetError::UnsupportedReference(part.to_owned()));
                }
                if recognized {
                    check_relative_resource(value)?;
                    resources.insert(value.to_owned());
                }
            }
        }
        Ok(resources)
    }

    /// Fail closed on missing files and symlinks or symlinked parent folders.
    /// This still does NOT pin a file's contents; production must verify SHA-256.
    pub fn validate_resource_tree(&self, root: &Path) -> Result<(), PresetError> {
        let root = root
            .canonicalize()
            .map_err(|_| PresetError::MissingRoot(root.display().to_string()))?;
        for relative in self.required_resources()? {
            let path = root.join(&relative);
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|_| PresetError::MissingResource(relative.clone()))?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(PresetError::UnsafeResource(relative));
            }
            let actual = path
                .canonicalize()
                .map_err(|_| PresetError::MissingResource(relative.clone()))?;
            if !actual.starts_with(&root) {
                return Err(PresetError::UnsafeResource(relative));
            }
        }
        Ok(())
    }
}

fn check_relative_resource(value: &str) -> Result<(), PresetError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains(':')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-/".contains(&byte))
        || Path::new(value)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(PresetError::UnsafeResource(value.to_owned()));
    }
    Ok(())
}

/// Mimic NetZapret's split at whitespace followed by another --[A-Za-z].
/// Spaces in --name=... and section labels must not be split.
fn split_arguments(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut tokens = Vec::new();
    let mut start = 0;
    for index in 1..bytes.len().saturating_sub(2) {
        if bytes[index] == b'-'
            && bytes[index + 1] == b'-'
            && bytes[index + 2].is_ascii_alphabetic()
            && bytes[index - 1].is_ascii_whitespace()
        {
            let token = line[start..index].trim();
            if !token.is_empty() {
                tokens.push(token);
            }
            start = index;
        }
    }
    let token = line[start..].trim();
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const UNIVERSAL: &str =
        include_str!("../../../../assets/desync/presets/Universal V13 Lite.txt");

    #[test]
    fn real_netzapret_v13_lite_preserves_all_sections_and_dependencies() {
        let preset = ZapretPreset::parse(UNIVERSAL).unwrap();
        assert_eq!(preset.name, "Universal V13 Lite");
        assert_eq!(preset.builtin_version.as_deref(), Some("2.26"));
        assert_eq!(preset.sections.len(), 73);
        let args = preset.command_arguments();
        assert_eq!(args.iter().filter(|a| a.as_str() == "--new").count(), 73);
        assert_eq!(args.iter().filter(|a| a.starts_with("--name=")).count(), 74);
        assert_eq!(preset.required_resources().unwrap().len(), 135);
    }

    #[test]
    fn authentic_source_sha256_is_pinned() {
        let actual = format!("{:x}", Sha256::digest(UNIVERSAL.as_bytes()));
        assert_eq!(actual, "e679901475d998cb0c7e5ed5c51a5286c3b97f02d4b6c0d947f4527830cacae7");
    }

    #[test]
    fn preserves_unknown_flags_section_names_and_multiple_switches() {
        let source = "# Preset: Test\n--lua-init=@lua/zapret-lib.lua\n--new\n--name=YouTube interface\n--lua-desync=fake:strategy=25 --lua-desync=multidisorder:strategy=25\n";
        let preset = ZapretPreset::parse(source).unwrap();
        assert_eq!(
            preset.command_arguments().iter().map(String::as_str).collect::<Vec<_>>(),
            vec![
                "--lua-init=@lua/zapret-lib.lua",
                "--new",
                "--name=YouTube interface",
                "--lua-desync=fake:strategy=25",
                "--lua-desync=multidisorder:strategy=25",
            ]
        );
    }

    #[test]
    fn fails_closed_on_unsafe_file_references() {
        for argument in [
            "--blob=x:@bin/../../private",
            "--hostlist=lists/../private.txt",
            "--lua-init=@C:/Windows/trick.lua",
            "--lua-init=@other/trick.lua",
        ] {
            let source = format!("# Preset: Test\n{argument}");
            assert!(ZapretPreset::parse(&source)
                .unwrap()
                .required_resources()
                .is_err());
        }
    }

    #[test]
    fn refuses_empty_sections_and_malformed_options() {
        for source in [
            "# Preset: Test\n--foo=1\n--new\n",
            "# Preset: Test\n--foo=1\n--new\n--new\n--bar=2",
            "# Preset: Test\nfoo=1",
        ] {
            assert!(ZapretPreset::parse(source).is_err());
        }
    }
}
