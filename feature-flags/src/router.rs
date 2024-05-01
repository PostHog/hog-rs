use std::future::ready;
use std::sync::Arc;

use axum::http::Method;
use axum::{
    routing::{get, post},
    Router,
};

use crate::{redis::Client, v0_endpoint};

#[derive(Clone)]
pub struct State {
    pub redis: Arc<dyn Client + Send + Sync>,
    // TODO: Add pgClient when ready
}

pub fn router<R: Client + Send + Sync + 'static>(redis: Arc<R>) -> Router {
    let state = State { redis };

    // // Very permissive CORS policy, as old SDK versions
    // // and reverse proxies might send funky headers.
    // let cors = CorsLayer::new()
    //     .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
    //     .allow_headers(AllowHeaders::mirror_request())
    //     .allow_credentials(true)
    //     .allow_origin(AllowOrigin::mirror_request());

    let router = Router::new()
        .route("/flags", post(v0_endpoint::flags).get(v0_endpoint::flags))
        .with_state(state);

    router
}
