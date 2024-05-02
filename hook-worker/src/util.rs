use crate::error::WebhookResponseError;
use futures::StreamExt;
use reqwest::Response;

pub async fn first_n_bytes_of_response(
    response: Response,
    n: usize,
) -> Result<String, WebhookResponseError> {
    let mut body = response.bytes_stream();
    let mut buffer = String::with_capacity(n);

    while let Some(chunk) = body.next().await {
        if buffer.len() >= n {
            // Early return before reading next chunk.
            break;
        }

        let chunk = chunk?;
        let chunk_str = std::str::from_utf8(&chunk)?;
        if let Some(partial_chunk_str) =
            chunk_str.get(0..std::cmp::min(n - buffer.len(), chunk_str.len()))
        {
            buffer.push_str(&partial_chunk_str);
        } else {
            // For whatever reason, we are out of bounds, give up.
            break;
        }
    }

    Ok(buffer)
}
