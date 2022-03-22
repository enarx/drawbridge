// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use drawbridge_core::http::{Request, Response, StatusCode};
use drawbridge_core::{async_trait, Handler};
use drawbridge_tree::{Memory, Service as Tree};

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
