// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::TreeContext;

use axum::response::IntoResponse;
use axum::Extension;
use log::warn;

pub async fn head(Extension(store): Extension<Arc<Store>>, tree: TreeContext) -> impl IntoResponse {
    store
        .tree(&tree)
        .get_meta()
        .await
        .map_err(|e| {
            warn!(target: "app::trees::head", "failed for `{tree}`: {:?}", e);
            e
        })
        .map(|meta| (meta, ()))
}
