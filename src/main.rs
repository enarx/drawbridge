// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use drawbridge::Service;
use drawbridge_http::http::Result;
use drawbridge_http::{Handler, IntoResponse};

use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;

#[async_std::main]
async fn main() -> Result<()> {
    let service = Service::default();

    let listener = TcpListener::bind(("127.0.0.1", 8080)).await?;
    eprintln!("LST: 127.0.0.1:8080");

    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;

        let service = service.clone();
        task::spawn(async move {
            eprintln!("CON: {}", stream.peer_addr()?);

            async_h1::accept(stream, |req| async {
                Ok(service.clone().handle(req).await.into_response().await)
            })
            .await
        });
    }

    Ok(())
}
