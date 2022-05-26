// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{error::Error, Create, CreateError, CreateItem, Get, GetError, Keys};

use std::collections::BTreeSet;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use drawbridge_type::digest::{Algorithm, Algorithms, ContentDigest};
use drawbridge_type::Meta;

use anyhow::{anyhow, Context as _};
use async_trait::async_trait;
use cap_async_std::fs::{Dir, DirEntry, File};
use cap_async_std::path::{Path, PathBuf};
use futures::future::join_all;
use futures::io::{self, copy, Cursor};
use futures::stream::{iter, Iter};
use futures::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use mime::Mime;
use serde::de::DeserializeOwned;
use serde::Serialize;

const ALGORITHM: Algorithm = Algorithm::Sha256;
const ALGORITHM_NAME: &str = "sha256";
const KEY_FILE: &str = "key.json";
const META_FILE: &str = "meta.json";
const CONTENT_LINK_FILE: &str = "content.bin";

/// Filesystem backed storage.
///
/// Layout:
///
///```text
///root/
///  storename/
///    sha256/
///      key/
///        (key.json checksum)/
///          key.json - unique
///          meta.json - possible duplicate
///          content.bin - symlink to value/*.bin file
///      value/
///        (checksum).bin - unique
///```
#[derive(Clone)]
pub struct Filesystem<K> {
    root: Dir,
    key_root: String,
    value_root: String,
    phantom: PhantomData<K>,
}

impl<K> Filesystem<K> {
    pub fn new(root: Dir, name: String) -> Self {
        Self {
            root,
            key_root: format!("store/v0/{}/{}/key", name, ALGORITHM_NAME),
            value_root: format!("store/v0/{}/{}/value", name, ALGORITHM_NAME),
            phantom: PhantomData,
        }
    }

    async fn get_key_dir(&self, expected_identity: &str) -> Result<PathBuf, Error>
    where
        K: Serialize + Sync + Send + Eq + Hash,
    {
        let key_dir: PathBuf = format!("{}/{}", self.key_root, expected_identity).into();

        if !self.root.exists(&key_dir).await {
            return Err(Error::NotFound);
        }

        let mut key_file = key_dir.clone();
        key_file.push(KEY_FILE);

        let actual_identity = sha256(&read(&mut open(&self.root, &key_file).await?).await?).await?;

        if expected_identity != actual_identity {
            return Err(Error::Internal(anyhow!("invalid key")));
        }

        Ok(key_dir)
    }
}

pub struct FilesystemCreateItem<'a, K> {
    fs: &'a mut Filesystem<K>,
    key_identity: String,
    mime: Mime,
    buf: Vec<u8>,
}

impl<'a, K> AsyncWrite for FilesystemCreateItem<'a, K> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.buf).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.buf).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.buf).poll_close(cx)
    }
}

#[async_trait]
impl<'a, K> CreateItem for FilesystemCreateItem<'a, K>
where
    K: Send + Sync,
{
    async fn finish(self) -> (u64, ContentDigest) {
        let size = self.buf.len() as _;

        // TODO: Compute hash while writing
        let mut buf: Vec<u8> = Vec::with_capacity(self.buf.len());
        let mut hasher = Algorithms::default().writer(&mut buf);
        copy(&mut self.buf.as_slice(), &mut hasher).await.unwrap();
        let hash = hasher.digests();

        // TODO: handle these errors properly (nothing from here down requires an unwrap): https://github.com/profianinc/drawbridge/issues/144
        let encoded_meta = serde_json::to_string(&Meta {
            hash: hash.clone(),
            size,
            mime: self.mime,
        })
        .with_context(|| "failed to serialize meta")
        .map_err(Error::Internal)
        .unwrap();

        write(
            &self.fs.root,
            &format!("{}/{}/{}", self.fs.key_root, self.key_identity, META_FILE),
            encoded_meta.as_bytes(),
        )
        .await
        .unwrap();

        self.fs
            .root
            .create_dir_all(&self.fs.value_root)
            .context("failed to create value root")
            .map_err(Error::Internal)
            .unwrap();

        let bin_link_path = format!(
            "{}/{}/{}",
            self.fs.key_root, self.key_identity, CONTENT_LINK_FILE
        );
        let bin_identity = sha256(&self.buf).await.unwrap();
        let bin_value_path = format!("{}/{}.bin", self.fs.value_root, bin_identity);

        if !self.fs.root.exists(&bin_value_path).await {
            write(&self.fs.root, &bin_value_path, &buf).await.unwrap();
        }

        self.fs
            .root
            .symlink(&format!("../../value/{}.bin", bin_identity), &bin_link_path)
            .await
            .unwrap();

        (size, hash)
    }
}

// TODO: abort and cleanup corrupted file structures properly: https://github.com/profianinc/drawbridge/issues/144
#[async_trait]
impl<K> Create<K> for Filesystem<K>
where
    K: Serialize + Sync + Send + Unpin + Eq + Hash,
{
    type Item<'a> = FilesystemCreateItem<'a, K> where K: 'a;
    type Error = Error;

    async fn create(
        &mut self,
        key: K,
        mime: Mime,
    ) -> Result<Self::Item<'_>, CreateError<Self::Error>> {
        let (encoded_key, identity) = encode_key(&key).await.map_err(CreateError::Internal)?;

        if self.get_key_dir(&identity).await.is_ok() {
            return Err(CreateError::Occupied);
        }

        self.root
            .create_dir_all(format!("{}/{}", self.key_root, identity))
            .context("failed to create key root")
            .map_err(Error::Internal)
            .map_err(CreateError::Internal)?;

        write(
            &self.root,
            &format!("{}/{}/{}", self.key_root, identity, KEY_FILE),
            encoded_key.as_bytes(),
        )
        .await
        .map_err(CreateError::Internal)?;

        Ok(FilesystemCreateItem {
            fs: self,
            key_identity: identity,
            mime,
            buf: vec![],
        })
    }
}

#[async_trait]
impl<K> Get<K> for Filesystem<K>
where
    K: Serialize + Sync + Send + Eq + Hash,
{
    type Item<'a> = Cursor<Vec<u8>> where K: 'a;
    type Error = Error;

    async fn get(&self, key: K) -> Result<(Meta, Self::Item<'_>), GetError<Self::Error>> {
        let (_encoded_key, identity) = encode_key(&key).await.map_err(Error::into_get_error)?;
        let key_dir = self
            .get_key_dir(&identity)
            .await
            .map_err(Error::into_get_error)?;
        let mut meta_file = key_dir.clone();
        meta_file.push(META_FILE);
        let mut bin_link_file = key_dir.clone();
        bin_link_file.push(CONTENT_LINK_FILE);

        let meta = read_json(
            &mut open(&self.root, &meta_file)
                .await
                .map_err(Error::into_get_error)?,
        )
        .await
        .with_context(|| format!("invalid meta file, identity: {}", identity))
        .map_err(|e| Error::Internal(e).into_get_error())?;
        let bin = read(
            &mut open(&self.root, &bin_link_file)
                .await
                .map_err(Error::into_get_error)?,
        )
        .await
        .with_context(|| format!("invalid bin file, identity: {}", identity))
        .map_err(|e| Error::Internal(e).into_get_error())?;
        Ok((meta, Cursor::new(bin)))
    }
}

#[async_trait]
impl<K> Keys<K> for Filesystem<K>
where
    K: DeserializeOwned + 'static + Sync + Send + Clone,
{
    type Stream = Iter<std::vec::IntoIter<Result<K, Error>>>;
    type StreamError = Error;

    async fn keys(&self) -> Self::Stream {
        let mut keys = vec![];

        for entry in iter_dir(&self.root, &self.key_root).await {
            for key_file in iter_dir_entry(&entry).await {
                if key_file.file_name().to_string_lossy().contains(KEY_FILE) {
                    let key_file = key_file.open().with_context(|| {
                        format!(
                            "failed to open key file: {}",
                            key_file.file_name().to_string_lossy()
                        )
                    });

                    keys.push(async {
                        let key_file = &mut key_file.map_err(Error::Internal)?;
                        read_json(key_file).await
                    });
                }
            }
        }

        iter(join_all(keys.into_iter()).await)
    }
}

async fn iter_dir<P: AsRef<Path> + fmt::Debug>(root: &Dir, path: P) -> Vec<DirEntry> {
    let mut entries = vec![];

    // TODO: handle errors here
    if let Ok(read_dir) = root.read_dir(&path).await {
        // TODO: handle errors here
        for dir_entry in read_dir.flatten() {
            entries.push(dir_entry);
        }
    }

    entries
}

async fn iter_dir_entry(dir_entry: &DirEntry) -> Vec<DirEntry> {
    let mut entries = vec![];

    // TODO: handle errors here
    if let Ok(dir) = dir_entry.open_dir() {
        // TODO: handle errors here
        if let Ok(read_dir) = dir.entries().await {
            // TODO: handle errors here
            for dir_entry in read_dir.flatten() {
                entries.push(dir_entry);
            }
        }
    }

    entries
}

async fn open<P: AsRef<Path> + fmt::Debug>(root: &Dir, path: P) -> Result<File, Error> {
    if !root.exists(&path).await {
        return Err(Error::NotFound);
    }

    root.open(&path)
        .await
        .with_context(|| format!("failed to open path {:?}", path))
        .map_err(Error::Internal)
}

async fn read<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Vec<u8>, Error> {
    let mut buffer = vec![];
    reader
        .read_to_end(&mut buffer)
        .await
        .context("failed to read bytes")
        .map_err(Error::Internal)?;
    Ok(buffer)
}

async fn read_json<T: DeserializeOwned, R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<T, Error> {
    serde_json::from_slice(&read(reader).await?)
        .context("failed to parse json")
        .map_err(Error::Internal)
}

/// This will also create the file if it does not exist.
async fn write<P: AsRef<Path> + fmt::Debug>(
    root: &Dir,
    path: P,
    buf: &[u8],
) -> Result<usize, Error> {
    if !root.exists(&path).await {
        root.create(&path)
            .await
            .with_context(|| format!("failed to create and open path: {:?}", path))
    } else {
        root.open(&path)
            .await
            .with_context(|| format!("failed to open path: {:?}", path))
    }
    .map_err(Error::Internal)?
    .write(buf)
    .await
    .context("failed to read")
    .map_err(Error::Internal)
}

async fn sha256(buffer: &[u8]) -> Result<String, Error> {
    let mut set = BTreeSet::new();
    set.insert(ALGORITHM);
    let mut hash_buffer: Vec<u8> = Vec::with_capacity(buffer.len());
    let mut hasher = Algorithms::from(set).writer(&mut hash_buffer);
    hasher
        .write_all(buffer)
        .await
        .context("failed to write hash to buffer")
        .map_err(Error::Internal)?;
    let digest = hasher
        .digests()
        .get(&ALGORITHM)
        // SAFETY: This unwrap should never fail as this is the same algo we just inserted.
        .unwrap()
        .iter()
        .map(|byte| format!("{:0x}", byte))
        .collect();
    Ok(digest)
}

async fn encode_key<K>(key: K) -> Result<(String, String), Error>
where
    K: Serialize,
{
    let key_json = serde_json::to_string(&key)
        .context("failed to serialize key")
        .map_err(Error::Internal)?;
    let identity = sha256(key_json.as_bytes()).await?;
    Ok((key_json, identity))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;

    use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
    use tempdir::TempDir;

    const TEST_DATA_DIRECTORY: &str = ".test_data";

    #[tokio::test]
    async fn test() {
        let key = "test".to_string();
        let mime = "text/plain".parse::<Mime>().unwrap();

        let dir = TempDir::new(TEST_DATA_DIRECTORY).unwrap();
        let root_dir = File::open(dir.path()).unwrap();

        let mut fs = Filesystem::new(Dir::from_std_file(root_dir.into()), "somestore".to_string());

        assert_eq!(
            format!(
                "{:?}",
                fs.get(key.clone())
                    .await
                    .map(|(x, y)| (x, y.into_inner()))
                    .unwrap_err()
            ),
            format!("{:?}", GetError::NotFound::<Error>)
        );

        assert!(fs.keys().await.collect::<Vec<_>>().await.is_empty());

        let hash = {
            let mut hasher = Algorithms::default().writer(io::sink());
            assert!(matches!(hasher.write_all(&[42]).await, Ok(())));
            hasher.digests()
        };

        {
            let w = fs.create(key.clone(), mime.clone()).await;
            assert!(matches!(w, Ok(_)));
            let mut w = w.unwrap();
            assert!(matches!(w.write_all(&[42]).await, Ok(())));
            assert_eq!(w.finish().await, (1, hash.clone()))
        }

        {
            let mr = fs.get(key.clone()).await;
            assert!(matches!(mr, Ok(_)));
            let mut v = vec![];
            let (meta, mut r) = mr.unwrap();
            assert_eq!(
                meta,
                Meta {
                    mime: mime.clone(),
                    size: 1,
                    hash,
                }
            );
            assert!(matches!(r.read_to_end(&mut v).await, Ok(1)))
        }

        assert!(matches!(
            fs.create(key.clone(), mime).await,
            Err(CreateError::Occupied)
        ));

        assert_eq!(
            fs.keys()
                .await
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .map(|k| k.unwrap())
                .collect::<Vec<String>>(),
            vec![key]
        );
    }
}
