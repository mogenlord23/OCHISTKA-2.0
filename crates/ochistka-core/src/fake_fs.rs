//! Дерево файлов в памяти для тестов ядра. Пути — строки с `\`, как в Windows.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::sync::Mutex;

use crate::fs::{CleanupError, DirEntry, FsPort, Meta};
use crate::policy::{file_name, parent_dir};

/// Родитель с учётом корня диска: у `D:\a` родитель `D:\`, а не `D:`.
fn parent(path: &str) -> Option<String> {
    parent_dir(path).map(|dir| {
        if dir.len() == 2 && dir.ends_with(':') {
            format!("{dir}\\")
        } else {
            dir.to_string()
        }
    })
}

#[derive(Clone)]
enum Node {
    Dir,
    File { bytes: Vec<u8>, modified: u64 },
    Link,
}

#[derive(Default)]
pub struct FakeFs {
    nodes: Mutex<BTreeMap<String, Node>>,
    pub reads: Mutex<u32>,
}

impl FakeFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dir(&self, path: &str) {
        self.insert_parents(path);
        self.nodes
            .lock()
            .unwrap()
            .insert(path.to_string(), Node::Dir);
    }

    pub fn file(&self, path: &str, bytes: &[u8], modified: u64) {
        self.insert_parents(path);
        self.nodes.lock().unwrap().insert(
            path.to_string(),
            Node::File {
                bytes: bytes.to_vec(),
                modified,
            },
        );
    }

    pub fn link(&self, path: &str) {
        self.insert_parents(path);
        self.nodes
            .lock()
            .unwrap()
            .insert(path.to_string(), Node::Link);
    }

    fn insert_parents(&self, path: &str) {
        let mut nodes = self.nodes.lock().unwrap();
        let mut current = parent(path);
        while let Some(dir) = current {
            nodes.entry(dir.clone()).or_insert(Node::Dir);
            current = parent(&dir);
        }
    }

    pub fn exists(&self, path: &str) -> bool {
        self.nodes.lock().unwrap().contains_key(path)
    }

    pub fn bytes_of(&self, path: &str) -> Option<Vec<u8>> {
        match self.nodes.lock().unwrap().get(path) {
            Some(Node::File { bytes, .. }) => Some(bytes.clone()),
            _ => None,
        }
    }
}

fn to_meta(node: &Node) -> Meta {
    match node {
        Node::Dir => Meta {
            is_dir: true,
            ..Meta::default()
        },
        Node::File { bytes, modified } => Meta {
            size: bytes.len() as u64,
            modified_unix: *modified,
            ..Meta::default()
        },
        Node::Link => Meta {
            is_symlink: true,
            ..Meta::default()
        },
    }
}

impl FsPort for FakeFs {
    fn metadata(&self, path: &str) -> Result<Meta, CleanupError> {
        self.nodes
            .lock()
            .unwrap()
            .get(path)
            .map(to_meta)
            .ok_or_else(|| CleanupError::NotFound(path.to_string()))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>, CleanupError> {
        let nodes = self.nodes.lock().unwrap();
        match nodes.get(path) {
            Some(Node::Dir) => {}
            _ => return Err(CleanupError::NotFound(path.to_string())),
        }
        *self.reads.lock().unwrap() += 1;
        Ok(nodes
            .iter()
            .filter(|(child, _)| parent(child).as_deref() == Some(path))
            .map(|(child, node)| DirEntry {
                name: file_name(child).to_string(),
                path: child.clone(),
                meta: to_meta(node),
            })
            .collect())
    }

    fn child(&self, dir: &str, name: &str) -> String {
        format!("{}\\{}", dir.trim_end_matches('\\'), name)
    }

    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, CleanupError> {
        match self.nodes.lock().unwrap().get(path) {
            Some(Node::File { bytes, .. }) => Ok(Box::new(Cursor::new(bytes.clone()))),
            _ => Err(CleanupError::NotFound(path.to_string())),
        }
    }

    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), CleanupError> {
        self.file(path, bytes, 0);
        Ok(())
    }

    fn create_dir_all(&self, path: &str) -> Result<(), CleanupError> {
        self.dir(path);
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> Result<(), CleanupError> {
        let mut nodes = self.nodes.lock().unwrap();
        let node = nodes
            .remove(from)
            .ok_or_else(|| CleanupError::NotFound(from.to_string()))?;
        drop(nodes);
        self.insert_parents(to);
        self.nodes.lock().unwrap().insert(to.to_string(), node);
        Ok(())
    }

    fn remove_file(&self, path: &str) -> Result<(), CleanupError> {
        match self.nodes.lock().unwrap().remove(path) {
            Some(Node::File { .. }) => Ok(()),
            _ => Err(CleanupError::NotFound(path.to_string())),
        }
    }

    fn remove_dir(&self, path: &str) -> Result<(), CleanupError> {
        let mut nodes = self.nodes.lock().unwrap();
        let has_children = nodes.keys().any(|key| parent(key).as_deref() == Some(path));
        if has_children {
            return Err(CleanupError::Io("папка не пуста".into()));
        }
        nodes.remove(path);
        Ok(())
    }
}
