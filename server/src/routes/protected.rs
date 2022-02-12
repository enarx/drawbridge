use axum::response::IntoResponse;

use crate::types::User;

/// This is just an example of how to implement endpoints behind OAuth.
pub async fn protected(user: User) -> impl IntoResponse {
    format!(
        "Welcome to the protected area\nHere's your info:\n{:?}",
        user
    )
}
