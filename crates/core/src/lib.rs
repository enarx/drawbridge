// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

use drawbridge_tree::{Memory, Service as Tree};
use drawbridge_web::http::{Request, Response, StatusCode};
use drawbridge_web::{async_trait, Handler};

#[derive(Clone, Default)]
pub struct Service {
    tree: Tree<Memory>,
}

#[async_trait]
impl Handler<()> for Service {
    type Response = Response;

    async fn handle(self, mut req: Request) -> Self::Response {
        let url = req.url_mut();
        let path = url.path().trim_start_matches('/');

        if path == "_tree" || path.starts_with("_tree/") {
            let path = format!("/{}", &path[5..].trim_start_matches('/'));
            url.set_path(&path);
            self.tree.handle(req).await
        } else {
            StatusCode::MethodNotAllowed.into()
        }
    }
}
