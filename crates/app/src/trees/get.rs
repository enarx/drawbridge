// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{Store, TrustedCertificate};

use drawbridge_type::TreeContext;

use async_std::sync::Arc;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use log::warn;

pub async fn get(
    Extension(store): Extension<Arc<Store>>,
    cert: Option<Extension<TrustedCertificate>>,
    tree: TreeContext,
) -> impl IntoResponse {
    if !cert.is_some() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "this operation requires either a valid client certificate signed by the Steward or a valid user session authentication",
        ).into_response());
    }

    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    store
        .tree(&tree)
        .get_to_writer(&mut body)
        .await
        .map_err(|e| {
            warn!(target: "app::trees::get", "failed for `{tree}`: {:?}", e);
            e.into_response()
        })
        .map(|meta| (meta, body))
}
