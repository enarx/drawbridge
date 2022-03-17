use tide::{Request, Response, Server, StatusCode};

pub fn new() -> Server<()> {
    async fn not_implemented<E>(_: Request<()>) -> Result<Response, E> {
        Ok(Response::builder(StatusCode::NotImplemented)
            .body("Auth handling not implemented")
            .build())
    }

    let mut srv = Server::new();
    srv.at("/").all(not_implemented);
    srv.at("/*").all(not_implemented);
    srv
}
