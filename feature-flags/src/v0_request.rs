use std::collections::{HashMap};
// use std::io::prelude::*;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
// use time::format_description::well_known::Iso8601;
// use time::OffsetDateTime;
use tracing::instrument;
// use uuid::Uuid;

use crate::api::FlagError;


#[derive(Deserialize, Default)]
pub struct FlagsQueryParams {

    #[serde(alias = "v")]
    pub version: Option<String>,

    #[serde(alias = "_")]
    sent_at: Option<i64>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct FlagRequest {
    #[serde(
        alias = "$token",
        alias = "api_key",
        skip_serializing_if = "Option::is_none"
    )]
    pub token: Option<String>,
    #[serde(alias = "$distinct_id", skip_serializing_if = "Option::is_none")]
    pub distinct_id: Option<String>,
    pub geoip_disable: Option<bool>,
    #[serde(default)]
    pub person_properties: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub groups: Option<HashMap<String, Value>>,
    // TODO: better type this since we know its going to be a nested json
    #[serde(default)]
    pub group_properties: Option<HashMap<String, Value>>,
    #[serde(alias = "$anon_distinct_id", skip_serializing_if = "Option::is_none")]
    pub anon_distinct_id: Option<String>,
}

impl FlagRequest {
    /// Takes a request payload and tries to decompress and unmarshall it.
    /// While posthog-js sends a compression query param, a sizable portion of requests
    /// fail due to it being missing when the body is compressed.
    /// Instead of trusting the parameter, we peek at the payload's first three bytes to
    /// detect gzip, fallback to uncompressed utf8 otherwise.
    #[instrument(skip_all)]
    pub fn from_bytes(bytes: Bytes) -> Result<FlagRequest, FlagError> {
        tracing::debug!(len = bytes.len(), "decoding new request");
        // TODO: Add base64 decoding
        let payload = String::from_utf8(bytes.into()).map_err(|e| {
                tracing::error!("failed to decode body: {}", e);
                FlagError::RequestDecodingError(String::from("invalid body encoding"))
            })?;

        tracing::debug!(json = payload, "decoded event data");
        Ok(serde_json::from_str::<FlagRequest>(&payload)?)
    }

    pub fn extract_and_verify_token(&self) -> Result<String, FlagError> {
        let token = match self {
            FlagRequest { token: Some(token), .. } => token.to_string(),
            _ => return Err(FlagError::NoTokenError),
        };
        // TODO: Get tokens from redis, confirm this one is valid
        // validate_token(&token)?;
        Ok(token)
    }

}


#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use bytes::Bytes;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use serde_json::json;

    use super::FlagError;
    use super::FlagRequest;

    #[test]
    fn extract_and_verify_token() {
        let parse_and_extract = |input: &'static str| -> Result<String, FlagError> {
            FlagRequest::from_bytes(input.into())
                .expect("failed to parse")
                .extract_and_verify_token()
        };

        let assert_extracted_token = |input: &'static str, expected: &str| {
            let id = parse_and_extract(input).expect("failed to extract");
            assert_eq!(id, expected);
        };

        // Return NoTokenError if not found
        assert!(matches!(
            parse_and_extract(r#"{"distinct_id": "xyz"}"#),
            Err(FlagError::NoTokenError)
        ));

        // Return TokenValidationError if token empty
        assert!(matches!(
            parse_and_extract(r#"{"api_key": "", "batch":[{"event": "e"}]}"#),
            Err(FlagError::TokenValidationError)
        ));

        // Return TokenValidationError if personal apikey
        assert!(matches!(
            parse_and_extract(r#"[{"event": "e", "token": "phx_hellothere"}]"#),
            Ok(_)
        ));


        // Return token from array if consistent
        assert_extracted_token(
            r#"[{"event":"e","token":"token1"},{"event":"e","token":"token1"}]"#,
            "token1",
        );

        // Return token from batch if present
        assert_extracted_token(
            r#"{"batch":[{"event":"e","token":"token1"}],"api_key":"batched"}"#,
            "batched",
        );

        // Return token from single event if present
        assert_extracted_token(r#"{"event":"e","$token":"single_token"}"#, "single_token");
        assert_extracted_token(r#"{"event":"e","api_key":"single_token"}"#, "single_token");
    }
}
