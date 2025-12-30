use crate::devto_api::Article;
use crate::error::AppError;
use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredArticle {
    pub key: String,
    pub id: Option<u64>,
    pub canonical_url: Option<String>,
    pub url: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub published_timestamp: Option<String>,
    pub published_at: Option<String>,
    pub edited_at: Option<String>,
    pub public_reactions_count: u32,
    pub positive_reactions_count: u32,
    pub tag_list: Vec<String>,
    pub user_name: Option<String>,
    pub user_username: Option<String>,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StateFile {
    pub items: Vec<StoredArticle>,
}

#[derive(Debug, Default)]
pub struct StateStore {
    pub items: HashMap<String, StoredArticle>,
}

impl StateStore {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .map_err(|e| AppError::feed(format!("state 読み込み失敗: {}", e)))?;
        let file: StateFile = serde_json::from_str(&content)
            .map_err(|e| AppError::feed(format!("state パース失敗: {}", e)))?;
        let mut store = StateStore::default();
        for item in file.items {
            store.items.insert(item.key.clone(), item);
        }
        Ok(store)
    }

    pub fn merge_from_api(&mut self, articles: &[Article], now: DateTime<Utc>) -> usize {
        let mut merged = 0;
        for article in articles {
            let Some(key) = article_key(article) else {
                eprintln!("記事の識別子が不足しているためスキップ: title={}", article.title);
                continue;
            };
            let stored = StoredArticle {
                key: key.clone(),
                id: article.id,
                canonical_url: article.canonical_url.clone(),
                url: article.url.clone(),
                title: article.title.clone(),
                description: article.description.clone(),
                published_timestamp: article.published_timestamp.clone(),
                published_at: article.published_at.clone(),
                edited_at: article.edited_at.clone(),
                public_reactions_count: article.public_reactions_count.unwrap_or(0),
                positive_reactions_count: article.positive_reactions_count.unwrap_or(0),
                tag_list: parse_tag_list(&article.tag_list),
                user_name: article.user.as_ref().and_then(|u| u.name.clone()),
                user_username: article.user.as_ref().and_then(|u| u.username.clone()),
                last_seen: now.to_rfc3339(),
            };
            self.items.insert(key, stored);
            merged += 1;
        }
        merged
    }

    pub fn prune(&mut self, now: DateTime<Utc>, max_days: u32, max_items: usize) {
        let cutoff = now - Duration::days(max_days as i64);
        self.items.retain(|_, item| {
            if let Some(dt) = parse_datetime(&item.last_seen) {
                dt >= cutoff
            } else {
                false
            }
        });

        if self.items.len() > max_items {
            // 古い順に落として上限を満たす
            let mut list: Vec<_> = self.items.values().cloned().collect();
            list.sort_by_key(|item| parse_datetime(&item.last_seen).unwrap_or_else(|| now));
            let keep = list.split_off(list.len().saturating_sub(max_items));
            self.items = keep.into_iter().map(|item| (item.key.clone(), item)).collect();
        }
    }

    pub fn to_sorted_vec(&self) -> Vec<StoredArticle> {
        let mut list: Vec<_> = self.items.values().cloned().collect();
        list.sort_by_key(|item| item.key.clone());
        list
    }

    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::feed(format!("state ディレクトリ作成失敗: {}", e)))?;
        }
        let file = StateFile {
            items: self.to_sorted_vec(),
        };
        let json = serde_json::to_string_pretty(&file)
            .map_err(|e| AppError::feed(format!("state 書き込み失敗: {}", e)))?;
        fs::write(path, json).map_err(|e| AppError::feed(format!("state 書き込み失敗: {}", e)))?;
        Ok(())
    }
}

pub fn article_key(article: &Article) -> Option<String> {
    if let Some(id) = article.id {
        return Some(id.to_string());
    }
    // id がない場合は canonical_url をキーにする
    article.canonical_url.clone()
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn select_updated_time(item: &StoredArticle) -> Option<DateTime<FixedOffset>> {
    let edited = item
        .edited_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
    let published = item
        .published_timestamp
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .or_else(|| {
            item.published_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        });

    match (edited, published) {
        (Some(a), Some(b)) => Some(if a > b { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn parse_tag_list(value: &Option<String>) -> Vec<String> {
    match value {
        Some(tags) => tags
            .split(',')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect(),
        None => Vec::new(),
    }
}
