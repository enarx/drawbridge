use axum::routing::any_service;
use axum::Router;

pub fn new() -> Router {
    Router::new()
        .nest("/_tree", any_service(tree::new()))
        .nest("/_tags", any_service(tags::new()))
    // TODO: Add auth
    // TODO: Figure out how to handle namespacing
}
