// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::{RepositoryName, TagName, TreePath};

use axum::response::IntoResponse;
use axum::Extension;

pub async fn get(
    Extension(store): Extension<Arc<Store>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(tag): Extension<TagName>,
    Extension(path): Extension<TreePath>,
) -> impl IntoResponse {
    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    store
        .repository(&repo)
        .tag(&tag)
        .path(&path)
        .get_to_writer(&mut body)
        .await
        .map_err(|e| {
            eprintln!(
                "Failed to GET path `{}` on tag `{}` of repository `{}`: {:?}",
                path, tag, repo, e
            );
            e
        })
        .map(|meta| (meta, body))
}
