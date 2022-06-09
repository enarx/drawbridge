// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::RepositoryContext;

use axum::response::IntoResponse;
use axum::Extension;

pub async fn get(
    Extension(store): Extension<Arc<Store>>,
    repo: RepositoryContext,
) -> impl IntoResponse {
    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    store
        .repository(&repo)
        .get_to_writer(&mut body)
        .await
        .map_err(|e| {
            eprintln!("Failed to GET repository `{}`: {:?}", repo, e);
            e
        })
        .map(|meta| (meta, body))
}
