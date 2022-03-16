use axum::routing::any_service;
use axum::Router;

use drawbridge_tags::new as tags;
use drawbridge_tree::new as tree;

pub fn new() -> Router {
    Router::new()
        .nest("/_tree", any_service(tree()))
        .nest("/_tags", any_service(tags()))
    // TODO: Add auth
    // TODO: Figure out how to handle namespacing
}
