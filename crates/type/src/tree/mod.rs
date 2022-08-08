// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod context;
mod directory;
mod entry;
mod name;
mod path;

pub use context::*;
pub use directory::*;
pub use entry::*;
pub use name::*;
pub use path::*;

use super::digest::Algorithms;
use super::Meta;

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::Seek;
use std::ops::Bound::{Excluded, Unbounded};
use std::ops::Deref;

use mime::APPLICATION_OCTET_STREAM;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub enum Content<F> {
    File(F),
    Directory(Vec<u8>),
}

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Tree<F>(BTreeMap<Path, Entry<Content<F>>>);

impl<F> Deref for Tree<F> {
    type Target = BTreeMap<Path, Entry<Content<F>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<F> IntoIterator for Tree<F> {
    type Item = (Path, Entry<Content<F>>);
    type IntoIter = std::collections::btree_map::IntoIter<Path, Entry<Content<F>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<F> Tree<F> {
    /// Returns an entry corresponding to the root of the tree
    pub fn root(&self) -> &Entry<Content<F>> {
        // SAFETY: A `Tree` can only be constructed via functionality
        // in this module and therefore always has a root.
        self.get(&Path::ROOT).unwrap()
    }
}

impl Tree<std::fs::File> {
    fn invalid_data_error(
        error: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> std::io::Error {
        use std::io;

        io::Error::new(io::ErrorKind::InvalidData, error)
    }

    pub fn from_path_sync(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let mut tree: BTreeMap<Path, Entry<Content<std::fs::File>>> = BTreeMap::new();
        WalkDir::new(&path)
            .contents_first(true)
            .follow_links(true)
            .into_iter()
            .try_for_each(|r| {
                let e = r?;

                let path = e.path().strip_prefix(&path).map_err(|e| {
                    Self::invalid_data_error(format!("failed to trim tree root path prefix: {e}",))
                })?;
                let path = path.to_str().ok_or_else(|| {
                    Self::invalid_data_error(format!(
                        "failed to convert tree path `{}` to Unicode",
                        path.to_string_lossy(),
                    ))
                })?;
                let path = path.parse().map_err(|err| {
                    Self::invalid_data_error(format!("failed to parse tree path `{path}`: {err}",))
                })?;

                let entry = match e.file_type() {
                    t if t.is_file() => {
                        let mut file = std::fs::File::open(e.path())?;
                        let (size, hash) = Algorithms::default().read_sync(&mut file)?;
                        file.rewind()?;
                        Entry {
                            meta: Meta {
                                hash,
                                size,
                                mime: match e.path().extension().and_then(OsStr::to_str) {
                                    Some("wasm") => "application/wasm".parse().unwrap(),
                                    Some("toml") => "application/toml".parse().unwrap(),
                                    _ => APPLICATION_OCTET_STREAM,
                                },
                            },
                            custom: Default::default(),
                            content: Content::File(file),
                        }
                    }
                    t if t.is_dir() => {
                        let dir: Directory<_> = tree
                            .range((Excluded(&path), Unbounded))
                            .map_while(|(p, e)| match p.split_last() {
                                Some((base, dir)) if dir == path.as_slice() => {
                                    // TODO: Remove the need for a clone, we probably should have
                                    // Path and PathBuf analogues for that
                                    Some((base.clone(), e))
                                }
                                _ => None,
                            })
                            .collect();
                        let buf = serde_json::to_vec(&dir).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("failed to encode directory to JSON: {e}",),
                            )
                        })?;
                        let (size, hash) = Algorithms::default().read_sync(&buf[..])?;
                        Entry {
                            meta: Meta {
                                hash,
                                size,
                                mime: Directory::<()>::TYPE.parse().unwrap(),
                            },
                            custom: Default::default(),
                            content: Content::Directory(buf),
                        }
                    }
                    _ => {
                        return Err(Self::invalid_data_error(format!(
                            "unsupported file type encountered at `{path}`",
                        )))
                    }
                };
                if tree.insert(path, entry).is_some() {
                    Err(Self::invalid_data_error("duplicate file name {name}"))
                } else {
                    Ok(())
                }
            })?;
        Ok(Self(tree))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{create_dir, write};
    use std::io::Read;

    use tempfile::tempdir;

    #[test]
    fn from_path_sync() {
        let root = tempdir().expect("failed to create temporary root directory");
        write(root.path().join("test-file-foo"), "foo").unwrap();

        create_dir(root.path().join("test-dir")).unwrap();
        write(root.path().join("test-dir").join("test-file-bar"), "bar").unwrap();

        let foo_meta = Algorithms::default()
            .read_sync("foo".as_bytes())
            .map(|(size, hash)| Meta {
                hash,
                size,
                mime: APPLICATION_OCTET_STREAM,
            })
            .unwrap();

        let bar_meta = Algorithms::default()
            .read_sync("bar".as_bytes())
            .map(|(size, hash)| Meta {
                hash,
                size,
                mime: APPLICATION_OCTET_STREAM,
            })
            .unwrap();

        let test_dir_json = serde_json::to_vec(&Directory::from({
            let mut m = BTreeMap::new();
            assert_eq!(
                m.insert(
                    "test-file-bar".parse().unwrap(),
                    Entry {
                        meta: bar_meta.clone(),
                        custom: Default::default(),
                        content: (),
                    },
                ),
                None
            );
            m
        }))
        .unwrap();
        let test_dir_meta = Algorithms::default()
            .read_sync(&test_dir_json[..])
            .map(|(size, hash)| Meta {
                hash,
                size,
                mime: Directory::<()>::TYPE.parse().unwrap(),
            })
            .unwrap();

        let root_json = serde_json::to_vec(&Directory::from({
            let mut m = BTreeMap::new();
            assert_eq!(
                m.insert(
                    "test-dir".parse().unwrap(),
                    Entry {
                        meta: test_dir_meta.clone(),
                        custom: Default::default(),
                        content: (),
                    },
                ),
                None
            );
            m
        }))
        .unwrap();
        let root_meta = Algorithms::default()
            .read_sync(&root_json[..])
            .map(|(size, hash)| Meta {
                hash,
                size,
                mime: Directory::<()>::TYPE.parse().unwrap(),
            })
            .unwrap();

        let tree = Tree::from_path_sync(root.path()).expect("failed to construct a tree");

        assert_eq!(tree.root().meta, root_meta);
        assert!(tree.root().custom.is_empty());
        assert!(matches!(tree.root().content, Content::Directory(ref json) if json == &root_json));

        let mut tree = tree.into_iter();

        let (path, entry) = tree.next().unwrap();
        assert_eq!(path, Path::ROOT);
        assert_eq!(entry.meta, root_meta);
        assert!(entry.custom.is_empty());
        assert!(matches!(entry.content, Content::Directory(json) if json == root_json));

        let (path, entry) = tree.next().unwrap();
        assert_eq!(path, "test-dir".parse().unwrap());
        assert_eq!(entry.meta, test_dir_meta);
        assert!(entry.custom.is_empty());
        assert!(matches!(entry.content, Content::Directory(json) if json == test_dir_json));

        let (path, entry) = tree.next().unwrap();
        assert_eq!(path, "test-dir/test-file-bar".parse().unwrap());
        assert_eq!(entry.meta, bar_meta);
        assert!(entry.custom.is_empty());
        assert!(matches!(entry.content, Content::File(_)));
        if let Content::File(mut file) = entry.content {
            let mut buf = vec![];
            assert_eq!(file.read_to_end(&mut buf).unwrap(), "bar".len());
            assert_eq!(buf, "bar".as_bytes());
        } else {
            panic!("invalid content type")
        }

        let (path, entry) = tree.next().unwrap();
        assert_eq!(path, "test-file-foo".parse().unwrap());
        assert_eq!(entry.meta, foo_meta);
        assert!(entry.custom.is_empty());
        assert!(matches!(entry.content, Content::File(_)));
        if let Content::File(mut file) = entry.content {
            let mut buf = vec![];
            assert_eq!(file.read_to_end(&mut buf).unwrap(), "foo".len());
            assert_eq!(buf, "foo".as_bytes());
        } else {
            panic!("invalid content type")
        }

        assert!(tree.next().is_none());
    }
}
