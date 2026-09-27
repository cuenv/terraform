use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::ACCEPT;

const MAX_ATTEMPTS: u8 = 4;
const RETRY_DELAY: Duration = Duration::from_secs(1);

pub fn client() -> Result<Client> {
    Client::builder()
        .user_agent(concat!("pulumi-cue/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .context("build HTTP client")
}

pub fn get(client: &Client, url: &str, bearer_token: Option<&str>) -> Result<Response> {
    for attempt in 0..MAX_ATTEMPTS {
        let mut request = client.get(url).header(ACCEPT, "application/json");
        if let Some(token) = bearer_token.filter(|token| !token.is_empty()) {
            request = request.bearer_auth(token);
        }

        match request.send() {
            Ok(response) if retryable_status(response.status()) && attempt + 1 < MAX_ATTEMPTS => {
                thread::sleep(RETRY_DELAY);
            }
            Ok(response) => return Ok(response),
            Err(error)
                if (error.is_connect() || error.is_timeout()) && attempt + 1 < MAX_ATTEMPTS =>
            {
                thread::sleep(RETRY_DELAY);
            }
            Err(error) => return Err(error).with_context(|| format!("GET {url}")),
        }
    }

    unreachable!("HTTP retry loop always returns the final response or error")
}

fn retryable_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}
