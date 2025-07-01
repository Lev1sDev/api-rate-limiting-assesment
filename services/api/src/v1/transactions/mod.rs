use axum::{routing::post, Router};

mod submit;

pub fn router() -> Router<crate::lib::AppState> {
    Router::new()
        .route("/submit", post(submit::handler))
}