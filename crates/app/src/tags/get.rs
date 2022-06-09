// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::TagContext;

use axum::response::IntoResponse;
use axum::Extension;

pub async fn get(Extension(store): Extension<Arc<Store>>, tag: TagContext) -> impl IntoResponse {
    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    store
        .tag(&tag)
        .get_to_writer(&mut body)
        .await
        .map_err(|e| {
            eprintln!("Failed to GET tag `{}`: {:?}", tag, e);
            e
        })
        .map(|meta| (meta, body))
}
