use futures::StreamExt;
use reqwest::Response;

pub async fn first_n_bytes_of_response(
    response: Response,
    n: usize,
) -> Result<String, reqwest::Error> {
    let mut body = response.bytes_stream().take(n);
    let mut buffer = String::new();

    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        buffer.push_str(std::str::from_utf8(&chunk).unwrap());
    }

    Ok(buffer)
}
