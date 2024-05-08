use std::sync::Arc;

use crate::{api::FlagError, redis::Client};

use serde::{Deserialize, Serialize};
use tracing::instrument;


// TRICKY: I'm still not sure where the :1: is coming from.
// The Django prefix is `posthog` only.
// It's from here: https://docs.djangoproject.com/en/4.2/topics/cache/#cache-versioning
// F&!£%% on the bright side we don't use this functionality yet.
// Will rely on integration tests to catch this.
const TEAM_TOKEN_CACHE_PREFIX: &str = "posthog:1:team_token:";

// TODO: Check what happens if json has extra stuff, does serde ignore it? Yes
// Make sure we don't serialize and store team data in redis. Let main decide endpoint control this...
// and track misses. Revisit if this becomes an issue.
// because otherwise very annoying to keep this in sync with main django which has a lot of extra fields we need here.
// will lead to inconsistent behaviour.
// This is turning out to be very annoying, because we have django key prefixes to be mindful of as well.
// Wonder if it would be better to make these caches independent? This generates that new problem of CRUD happening in Django,
// which needs to update this cache immediately, so they can't really ever be independent.
// True for both team cache and flags cache. Hmm. Just I guess need to add tests around the key prefixes...
#[derive(Debug, Deserialize, Serialize)]
pub struct Team {
    pub id: i64,
    pub name: String,
    pub api_token: String,
}

impl Team {
    /// Validates a token, and returns a team if it exists.
    /// 
    
    #[instrument(skip_all)]
    pub async fn from_redis(
        client: Arc<dyn Client + Send + Sync>,
        token: String,
    ) -> Result<Team, FlagError> {

        // TODO: Instead of failing here, i.e. if not in redis, fallback to pg
        let serialized_team = client
            .get(
                format!("{TEAM_TOKEN_CACHE_PREFIX}{}", token)
            )
            .await
            .map_err(|e| {
                tracing::error!("failed to fetch data: {}", e);
                // TODO: Can be other errors if serde_pickle destructuring fails?
                FlagError::TokenValidationError
            })?;
        
        let team: Team = serde_json::from_str(&serialized_team).map_err(|e| {
            tracing::error!("failed to parse data to team: {}", e);
            // TODO: Internal error, shouldn't send back to client
            FlagError::RequestParsingError(e)
        })?;

        Ok(team)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use anyhow::Error;

    use crate::redis::RedisClient;
    use rand::{distributions::Alphanumeric, Rng};

    use super::*;

    fn random_string(prefix: &str, length: usize) -> String {
        let suffix: String = rand::thread_rng()
            .sample_iter(Alphanumeric)
            .take(length)
            .map(char::from)
            .collect();
        format!("{}{}", prefix, suffix)
    }

    async fn insert_new_team_in_redis(client: Arc<RedisClient>) -> Result<Team, Error> {
        let id = rand::thread_rng().gen_range(0..10_000_000);
        let token = random_string("phc_", 12);
        let team = Team {
            id: id,
            name: "team".to_string(),
            api_token: token,
        };

        let serialized_team = serde_json::to_string(&team)?;
        client
            .set(
                format!("{TEAM_TOKEN_CACHE_PREFIX}{}", team.api_token.clone()),
                serialized_team,
            )
            .await?;

        Ok(team)
    }

    #[tokio::test]
    async fn test_fetch_team_from_redis() {
        let client = RedisClient::new("redis://localhost:6379/".to_string())
            .expect("Failed to create redis client");
        let client = Arc::new(client);

        let team = insert_new_team_in_redis(client.clone()).await.unwrap();

        let target_token = team.api_token;

        let team_from_redis = Team::from_redis(client.clone(), target_token.clone()).await.unwrap();
        assert_eq!(
            team_from_redis.api_token, target_token
        );
        assert_eq!(
            team_from_redis.id, team.id
        );
    }

    #[tokio::test]
    async fn test_fetch_invalid_team_from_redis() {
        let client = RedisClient::new("redis://localhost:6379/".to_string())
            .expect("Failed to create redis client");
        let client = Arc::new(client);

        match Team::from_redis(client.clone(), "banana".to_string()).await {
            Err(FlagError::TokenValidationError) => (),
            _ => panic!("Expected TokenValidationError"),
        };
    }
}
