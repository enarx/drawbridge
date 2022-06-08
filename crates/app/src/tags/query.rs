// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::RepositoryName;

use axum::response::IntoResponse;
use axum::{Extension, Json};

pub async fn query(
    Extension(store): Extension<Arc<Store>>,
    Extension(repo): Extension<RepositoryName>,
) -> impl IntoResponse {
    store
        .repository(&repo)
        .tags()
        .await
        .map_err(|e| {
            eprintln!("Failed to GET tags on repository `{}`: {:?}", repo, e);
            e
        })
        .map(Json)
}
