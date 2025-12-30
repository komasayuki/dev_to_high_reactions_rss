use crate::config::AppConfig;
use crate::error::AppError;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use std::thread::sleep;
use std::time::Duration;

const BASE_URL: &str = "https://dev.to/api/articles";
const MAX_RETRIES: usize = 3;
const TIMEOUT_SECS: u64 = 15;

#[derive(Debug, Clone, Deserialize)]
pub struct Article {
    pub id: Option<u64>,
    pub title: String,
    pub url: Option<String>,
    pub canonical_url: Option<String>,
    pub description: Option<String>,
    pub published_timestamp: Option<String>,
    pub published_at: Option<String>,
    pub edited_at: Option<String>,
    pub public_reactions_count: Option<u32>,
    pub positive_reactions_count: Option<u32>,
    pub tag_list: Option<String>,
    pub user: Option<User>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub name: Option<String>,
    pub username: Option<String>,
}

pub fn fetch_articles(config: &AppConfig) -> Result<Vec<Article>, AppError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::network(format!("HTTP クライアント作成失敗: {}", e)))?;

    let mut all_articles = Vec::new();
    for page in 1..=config.max_pages {
        let articles = fetch_page(&client, config, page)?;
        if articles.is_empty() {
            break;
        }
        let count = articles.len();
        all_articles.extend(articles);
        if count < config.per_page as usize {
            break;
        }
    }
    Ok(all_articles)
}

fn fetch_page(client: &Client, config: &AppConfig, page: u32) -> Result<Vec<Article>, AppError> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        // 一時的な失敗に備えてリトライする
        let request = client
            .get(BASE_URL)
            .query(&[
                ("top", config.lookback_days.to_string()),
                ("per_page", config.per_page.to_string()),
                ("page", page.to_string()),
            ]);
        let url = format!(
            "{}?top={}&per_page={}&page={}",
            BASE_URL, config.lookback_days, config.per_page, page
        );

        let response = request.send();
        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    let parsed: Vec<Article> = resp.json().map_err(|e| {
                        AppError::network(format!("API JSON パース失敗: {}", e))
                    })?;
                    return Ok(parsed);
                }

                if should_retry(status) && attempt < MAX_RETRIES {
                    let backoff = backoff_duration(attempt);
                    eprintln!(
                        "API リトライ: url={} status={} attempt={} backoff={}s",
                        url,
                        status,
                        attempt,
                        backoff.as_secs()
                    );
                    sleep(backoff);
                    continue;
                }

                return Err(AppError::network(format!(
                    "API 失敗: url={} status={} attempt={}",
                    url, status, attempt
                )));
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    let backoff = backoff_duration(attempt);
                    eprintln!(
                        "API リトライ: url={} error={} attempt={} backoff={}s",
                        url,
                        e,
                        attempt,
                        backoff.as_secs()
                    );
                    sleep(backoff);
                    continue;
                }
                return Err(AppError::network(format!(
                    "API 失敗: url={} error={} attempt={}",
                    url, e, attempt
                )));
            }
        }
    }
}

fn should_retry(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

fn backoff_duration(attempt: usize) -> Duration {
    let secs = 2u64.pow((attempt as u32).saturating_sub(1));
    Duration::from_secs(secs)
}
