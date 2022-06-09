// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::digest::Algorithms;
use drawbridge_type::{Meta, RepositoryContext};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use mime::APPLICATION_JSON;

pub async fn query(
    Extension(store): Extension<Arc<Store>>,
    repo: RepositoryContext,
) -> impl IntoResponse {
    {
        let tags = store
            .repository(&repo)
            .tags()
            .await
            .map_err(IntoResponse::into_response)?;
        // TODO: Optimize hash computation
        let buf = serde_json::to_vec(&tags).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encode tags as JSON: {e}"),
            )
                .into_response()
        })?;
        let hash = Algorithms::default().read_sync(&buf[..]).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to compute tag digest: {e}"),
            )
                .into_response()
        })?;
        Ok::<_, Response>((
            Meta {
                hash,
                size: buf.len() as _,
                mime: APPLICATION_JSON,
            },
            buf,
        ))
    }
    .map_err(|e| {
        eprintln!("Failed to GET tags on `{}`: {:?}", repo, e);
        e
    })
}
