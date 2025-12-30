use clap::Parser;
use chrono::{DateTime, FixedOffset, Utc};
use dev_to_high_reactions_rss::atom::{build_feed_xml, default_feed_updated, FeedEntry, FeedInfo};
use dev_to_high_reactions_rss::config::AppConfig;
use dev_to_high_reactions_rss::devto_api::fetch_articles;
use dev_to_high_reactions_rss::error::AppError;
use dev_to_high_reactions_rss::html::{build_index_html, IndexPage};
use dev_to_high_reactions_rss::state::{select_updated_time, StateStore, StoredArticle};
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "devto-feed", version)]
struct Cli {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    state: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    index: PathBuf,
    #[arg(long = "last-build")]
    last_build: PathBuf,
    #[arg(long)]
    dry_run: bool,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(err.exit_code());
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::parse();
    let config = AppConfig::load(&cli.config)?;
    let now = Utc::now();

    let mut state = StateStore::load(&cli.state)?;
    let articles = fetch_articles(&config)?;
    let filtered: Vec<_> = articles
        .into_iter()
        .filter(|item| item.public_reactions_count.unwrap_or(0) >= config.min_reactions)
        .collect();

    let merged = state.merge_from_api(&filtered, now);
    state.prune(now, config.max_stored_days, config.max_stored_items);

    let mut items: Vec<StoredArticle> = state
        .items
        .values()
        .cloned()
        .filter(|item| item.public_reactions_count >= config.min_reactions)
        .collect();

    // reactions 降順 -> 公開日降順で並べる
    items.sort_by(|a, b| compare_items(a, b));
    if items.len() > config.max_feed_entries {
        items.truncate(config.max_feed_entries);
    }

    let site_url = config.site_url.clone();
    let feed_url = build_url(&site_url, &config.feed_path);
    let index_url = if site_url.is_empty() {
        "index.html".to_string()
    } else {
        build_url(&site_url, "index.html")
    };

    let entries = build_entries(&items, now);
    let feed_updated = default_feed_updated(&entries, now);
    let feed_id = if site_url.is_empty() {
        format!("tag:dev.to,{}:devto-feed", now.format("%Y"))
    } else {
        feed_url.clone()
    };
    let feed = FeedInfo {
        id: feed_id,
        title: config.site_title.clone(),
        description: config.site_description.clone(),
        updated: feed_updated,
        feed_url: feed_url.clone(),
        index_url: index_url.clone(),
        entries,
    };
    let feed_xml = build_feed_xml(&feed)?;

    let index_page = IndexPage {
        title: config.site_title.clone(),
        description: config.site_description.clone(),
        feed_url,
        updated: feed_updated,
        min_reactions: config.min_reactions,
        lookback_days: config.lookback_days,
    };
    let index_html = build_index_html(&index_page);

    if cli.dry_run {
        println!("dry-run: merged={} stored={} entries={}", merged, state.items.len(), feed.entries.len());
        return Ok(());
    }

    write_output(&cli.out, &feed_xml)?;
    write_output(&cli.index, &index_html)?;
    write_output(&cli.last_build, &now.to_rfc3339())?;
    write_nojekyll(&cli.out)?;
    state.save(&cli.state)?;

    Ok(())
}

fn compare_items(a: &StoredArticle, b: &StoredArticle) -> Ordering {
    let reactions = b
        .public_reactions_count
        .cmp(&a.public_reactions_count);
    if reactions != Ordering::Equal {
        return reactions;
    }

    let a_published = published_time(a);
    let b_published = published_time(b);
    b_published.cmp(&a_published)
}

fn published_time(item: &StoredArticle) -> Option<DateTime<FixedOffset>> {
    item.published_timestamp
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .or_else(|| {
            item.published_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        })
}

fn build_entries(items: &[StoredArticle], now: DateTime<Utc>) -> Vec<FeedEntry> {
    items
        .iter()
        .filter_map(|item| {
            let updated = select_updated_time(item)
                .or_else(|| published_time(item))
                .unwrap_or_else(|| now.with_timezone(&FixedOffset::east_opt(0).unwrap()));
            let id = build_entry_id(item, now);
            let link = item
                .canonical_url
                .clone()
                .or_else(|| item.url.clone())?;
            let summary_html = build_summary_html(item);
            Some(FeedEntry {
                id,
                title: item.title.clone(),
                link,
                updated,
                summary_html,
            })
        })
        .collect()
}

fn build_entry_id(item: &StoredArticle, now: DateTime<Utc>) -> String {
    if let Some(id) = item.id {
        return format!("tag:dev.to,{}:{}", now.format("%Y"), id);
    }
    item.canonical_url
        .clone()
        .unwrap_or_else(|| format!("tag:dev.to,{}:unknown", now.format("%Y")))
}

fn build_summary_html(item: &StoredArticle) -> String {
    let reactions = format!(
        "Reactions: public {} / positive {}",
        item.public_reactions_count, item.positive_reactions_count
    );
    let author = match (&item.user_name, &item.user_username) {
        (Some(name), Some(username)) => format!(
            "Author: <a href=\"https://dev.to/{username}\">{name}</a>",
            name = name,
            username = username
        ),
        (Some(name), None) => format!("Author: {}", name),
        _ => "Author: unknown".to_string(),
    };
    let date = item
        .published_timestamp
        .as_deref()
        .or(item.published_at.as_deref())
        .unwrap_or("unknown");
    let tags = if item.tag_list.is_empty() {
        "Tags: none".to_string()
    } else {
        format!("Tags: {}", item.tag_list.join(", "))
    };
    let description = item
        .description
        .as_deref()
        .map(|d| format!("Description: {}", d))
        .unwrap_or_else(|| "Description: none".to_string());

    format!(
        "{}<br/>{}<br/>Published: {}<br/>{}<br/>{}",
        reactions, author, date, tags, description
    )
}

fn build_url(base: &str, path: &str) -> String {
    if base.is_empty() {
        return path.to_string();
    }
    format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'))
}

fn write_output(path: &PathBuf, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::feed(format!("出力ディレクトリ作成失敗: {}", e)))?;
    }
    fs::write(path, content)
        .map_err(|e| AppError::feed(format!("出力書き込み失敗: {}", e)))?;
    Ok(())
}

fn write_nojekyll(out_path: &PathBuf) -> Result<(), AppError> {
    let Some(parent) = out_path.parent() else {
        return Ok(());
    };
    let nojekyll_path = parent.join(".nojekyll");
    if nojekyll_path.exists() {
        return Ok(());
    }
    fs::write(&nojekyll_path, "")
        .map_err(|e| AppError::feed(format!(".nojekyll 作成失敗: {}", e)))?;
    Ok(())
}
