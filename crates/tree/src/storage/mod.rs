// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

pub use memory::Memory;

use crate::{meta::Meta, node::Node, path::Path};

use drawbridge_web::http::{Body, StatusCode};
use drawbridge_web::{async_trait, IntoResponse};

use async_std::io::Read;

#[async_trait]
pub trait Storage: Send + Sync {
    type Error: IntoResponse + From<StatusCode>;

    async fn roots(&self) -> Result<Vec<Node>, Self::Error>;
    async fn wants(&self, path: Path) -> Result<Vec<Node>, Self::Error>;

    async fn del(&self, path: Path) -> Result<(), Self::Error>;
    async fn get(&self, path: Path) -> Result<(Meta, Body), Self::Error>;
    async fn put<T>(&self, path: Path, meta: Meta, body: T) -> Result<(), Self::Error>
    where
        T: Send + Read + Unpin;
}

#[cfg(test)]
mod tests {
    use async_std::io::{copy, sink};
    use drawbridge_hash::Hash;

    use crate::{meta::Meta, node::Node, path::Path};

    use super::{Memory, Storage};

    async fn prep(mime: &str, mut data: &[u8]) -> (Node, Meta) {
        const HASH: &str = "sha256:LCa0a2j_xo_5m0U8HTBBNBNCLXBkg7-g-YpeiGJm564";

        let size = data.len() as u64;

        let mut wrtr = HASH.parse::<Hash>().unwrap().writer(sink());
        copy(&mut data, &mut wrtr).await.unwrap();

        let meta = Meta {
            hash: wrtr.finish().into(),
            mime: mime.parse().unwrap(),
            size,
        };

        let json = serde_json::to_vec(&meta).unwrap();
        let mut wrtr = meta.hash.writer(sink());
        copy(&mut &json[..], &mut wrtr).await.unwrap();

        (wrtr.finish().into(), meta)
    }

    #[async_std::test]
    async fn basic() {
        let cdata = &b"foo"[..];
        let (cnode, cmeta) = prep("application/octet-stream", cdata).await;

        let pdata = format!(r#"{{"foo":"{}"}}"#, cnode);
        let pdata = pdata.as_bytes();
        let (pnode, pmeta) = prep(crate::DIRECTORY, pdata).await;

        let ppath: Path = format!("/{}", pnode).parse().unwrap();
        let cpath: Path = format!("/{}/{}", pnode, cnode).parse().unwrap();

        // The default is no roots.
        let m = Memory::default();
        assert_eq!(m.roots().await.unwrap(), vec![]);

        // Create a parent object. This should create one root.
        m.put(ppath.clone(), pmeta.clone(), pdata).await.unwrap();
        assert_eq!(m.roots().await.unwrap(), vec![pnode.clone()]);

        // We should also be able to see the files wanted for upload.
        assert_eq!(m.wants(ppath.clone()).await.unwrap(), vec![cnode]);

        // Validate that we can fetch a partial upload.
        let (meta, data) = m.get(ppath.clone()).await.unwrap();
        assert_eq!(pmeta, meta);
        assert_eq!(pdata, data.into_bytes().await.unwrap());

        // Validate that we cannot fetch an incomplete tree item.
        m.get(cpath.clone()).await.unwrap_err();

        // Upload the child item. Roots don't change.
        m.put(cpath.clone(), cmeta.clone(), cdata).await.unwrap();
        assert_eq!(m.roots().await.unwrap(), vec![pnode]);

        // The wanted output should now be empty.
        assert_eq!(m.wants(ppath.clone()).await.unwrap(), vec![]);

        // Validate that we can fetch the upload.
        let (meta, data) = m.get(cpath.clone()).await.unwrap();
        assert_eq!(cmeta, meta);
        assert_eq!(cdata, data.into_bytes().await.unwrap());

        // Delete the parent and validate the tree is empty.
        m.del(ppath).await.unwrap();
        assert_eq!(m.roots().await.unwrap(), vec![]);
    }
}
