use tide::{Response, Server, StatusCode};

use drawbridge_tags::new as tags;
use drawbridge_tree::new as tree;

pub fn new() -> Server<()> {
    // TODO: Add auth
    // TODO: Add namespacing
    let mut srv = Server::new();
    srv.at("/_tags").nest(tags());
    srv.at("/_tags/").nest(tags());
    srv.at("/_tree").nest(tree());
    srv.at("/_tree/").nest(tree());
    srv.at("/*").all(|_| async move {
        Ok(Response::builder(StatusCode::NotFound)
            .body("Route not found")
            .build())
    });
    srv
}
