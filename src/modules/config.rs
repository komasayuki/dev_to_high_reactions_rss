use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub min_reactions: u32,
    pub lookback_days: u32,
    pub per_page: u32,
    pub max_pages: u32,
    pub max_feed_entries: usize,
    pub max_stored_days: u32,
    pub max_stored_items: usize,
    pub site_title: String,
    pub site_description: String,
    pub site_url: String,
    pub feed_path: String,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let content = fs::read_to_string(path)
            .map_err(|e| AppError::config(format!("config 読み込み失敗: {}", e)))?;
        let mut cfg: AppConfig = serde_yaml::from_str(&content)
            .map_err(|e| AppError::config(format!("config パース失敗: {}", e)))?;
        cfg.apply_env_overrides()?;
        cfg.validate()?;
        cfg.ensure_site_url();
        Ok(cfg)
    }

    fn apply_env_overrides(&mut self) -> Result<(), AppError> {
        if let Some(value) = get_env_non_empty("MIN_REACTIONS") {
            self.min_reactions = value
                .parse::<u32>()
                .map_err(|_| AppError::config("MIN_REACTIONS が数値ではありません"))?;
        }
        if let Some(value) = get_env_non_empty("LOOKBACK_DAYS") {
            self.lookback_days = value
                .parse::<u32>()
                .map_err(|_| AppError::config("LOOKBACK_DAYS が数値ではありません"))?;
        }
        if let Some(value) = get_env_non_empty("SITE_URL") {
            self.site_url = value;
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), AppError> {
        if self.per_page == 0 {
            return Err(AppError::config("per_page は 1 以上で指定してください"));
        }
        if self.max_pages == 0 {
            return Err(AppError::config("max_pages は 1 以上で指定してください"));
        }
        if self.max_feed_entries == 0 {
            return Err(AppError::config(
                "max_feed_entries は 1 以上で指定してください",
            ));
        }
        if self.max_stored_days == 0 {
            return Err(AppError::config(
                "max_stored_days は 1 以上で指定してください",
            ));
        }
        if self.max_stored_items == 0 {
            return Err(AppError::config(
                "max_stored_items は 1 以上で指定してください",
            ));
        }
        Ok(())
    }

    fn ensure_site_url(&mut self) {
        if !self.site_url.trim().is_empty() {
            self.site_url = normalize_site_url(&self.site_url);
            return;
        }
        if let Ok(repo) = env::var("GITHUB_REPOSITORY") {
            if let Some(url) = derive_site_url(&repo) {
                self.site_url = url;
            }
        }
    }
}

fn get_env_non_empty(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(value) => {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        }
        Err(_) => None,
    }
}

fn normalize_site_url(value: &str) -> String {
    let trimmed = value.trim_end_matches('/');
    trimmed.to_string()
}

fn derive_site_url(repo: &str) -> Option<String> {
    let mut parts = repo.split('/');
    let owner = parts.next()?;
    let name = parts.next()?;
    if name == format!("{}.github.io", owner) {
        return Some(format!("https://{}.github.io", owner));
    }
    Some(format!("https://{}.github.io/{}", owner, name))
}
