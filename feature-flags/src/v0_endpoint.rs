use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use axum::{debug_handler, Json};
use bytes::Bytes;
// TODO: stream this instead
use axum::extract::{MatchedPath, Query, State};
use axum::http::{HeaderMap, Method};
use axum_client_ip::InsecureClientIp;
use base64::Engine;
use tracing::instrument;

use crate::{
    api::{FlagError, FlagsResponse},
    router,
    v0_request::{FlagRequest, FlagsQueryParams},
};

/// Feature flag evaluation endpoint.
/// Only supports a specific shape of data, and rejects any malformed data.

#[instrument(
    skip_all,
    fields(
        path,
        token,
        batch_size,
        user_agent,
        content_encoding,
        content_type,
        version,
        compression,
        historical_migration
    )
)]
#[debug_handler]
pub async fn flags(
    state: State<router::State>,
    InsecureClientIp(ip): InsecureClientIp,
    meta: Query<FlagsQueryParams>,
    headers: HeaderMap,
    method: Method,
    path: MatchedPath,
    body: Bytes,
) -> Result<Json<FlagsResponse>, FlagError> {
    let user_agent = headers
        .get("user-agent")
        .map_or("unknown", |v| v.to_str().unwrap_or("unknown"));
    let content_encoding = headers
        .get("content-encoding")
        .map_or("unknown", |v| v.to_str().unwrap_or("unknown"));

    tracing::Span::current().record("user_agent", user_agent);
    tracing::Span::current().record("content_encoding", content_encoding);
    // tracing::Span::current().record("version", meta.lib_version.clone());
    tracing::Span::current().record("method", method.as_str());
    tracing::Span::current().record("path", path.as_str().trim_end_matches('/'));

    let request = match headers
        .get("content-type")
        .map_or("", |v| v.to_str().unwrap_or(""))
    {
        "application/x-www-form-urlencoded" => {
            return Err(FlagError::RequestDecodingError(String::from(
                "invalid form data",
            )));
        }
        ct => {
            tracing::Span::current().record("content_type", ct);

            FlagRequest::from_bytes(body)
        }
    }?;

    let token = request.extract_and_verify_token()?;

    tracing::Span::current().record("token", &token);

    tracing::debug!("request: {:?}", request);

    // TODO: Some actual processing for evaluating the feature flag

    Ok(Json(FlagsResponse {
        error_while_computing_flags: false,
        feature_flags: HashMap::from([
            ("beta-feature".to_string(), "variant-1".to_string()),
            ("rollout-flag".to_string(), true.to_string()),
        ]),
    }))
}
