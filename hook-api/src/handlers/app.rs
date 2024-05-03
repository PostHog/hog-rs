use axum::{extract::DefaultBodyLimit, routing, Router};

use hook_common::pgqueue::PgQueue;

use super::webhook;

pub fn add_routes(router: Router, pg_pool: PgQueue) -> Router {
    router
        .route("/", routing::get(index))
        .route("/_readiness", routing::get(index))
        .route("/_liveness", routing::get(index)) // No async loop for now, just check axum health
        .route(
            "/webhook",
            routing::post(webhook::post)
                .with_state(pg_pool)
                .layer(DefaultBodyLimit::disable()),
        )
}

pub async fn index() -> &'static str {
    "rusty-hook api"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use hook_common::pgqueue::PgQueue;
    use http_body_util::BodyExt; // for `collect`
    use sqlx::PgPool;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[sqlx::test(migrations = "../migrations")]
    async fn index(db: PgPool) {
        let pg_queue = PgQueue::new_from_pool("test_index", db).await;

        let app = add_routes(Router::new(), pg_queue);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"rusty-hook api");
    }
}
