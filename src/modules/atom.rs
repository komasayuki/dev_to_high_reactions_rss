use crate::error::AppError;
use chrono::{DateTime, FixedOffset, Utc};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct FeedEntry {
    pub id: String,
    pub title: String,
    pub link: String,
    pub updated: DateTime<FixedOffset>,
    pub summary_html: String,
}

#[derive(Debug, Clone)]
pub struct FeedInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub updated: DateTime<FixedOffset>,
    pub feed_url: String,
    pub index_url: String,
    pub entries: Vec<FeedEntry>,
}

pub fn build_feed_xml(feed: &FeedInfo) -> Result<String, AppError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| AppError::feed(format!("XML 宣言失敗: {}", e)))?;

    let mut feed_start = BytesStart::new("feed");
    feed_start.push_attribute(("xmlns", "http://www.w3.org/2005/Atom"));
    writer
        .write_event(Event::Start(feed_start))
        .map_err(|e| AppError::feed(format!("feed 開始失敗: {}", e)))?;

    write_text_element(&mut writer, "id", &feed.id)?;
    write_text_element(&mut writer, "title", &feed.title)?;
    write_text_element(&mut writer, "updated", &feed.updated.to_rfc3339())?;

    write_link(&mut writer, "self", &feed.feed_url)?;
    write_link(&mut writer, "alternate", &feed.index_url)?;

    if !feed.description.trim().is_empty() {
        write_text_element(&mut writer, "subtitle", &feed.description)?;
    }

    for entry in &feed.entries {
        writer
            .write_event(Event::Start(BytesStart::new("entry")))
            .map_err(|e| AppError::feed(format!("entry 開始失敗: {}", e)))?;

        write_text_element(&mut writer, "id", &entry.id)?;
        write_text_element(&mut writer, "title", &entry.title)?;
        write_link(&mut writer, "alternate", &entry.link)?;
        write_text_element(&mut writer, "updated", &entry.updated.to_rfc3339())?;

        let mut summary = BytesStart::new("summary");
        summary.push_attribute(("type", "html"));
        writer
            .write_event(Event::Start(summary))
            .map_err(|e| AppError::feed(format!("summary 開始失敗: {}", e)))?;
        // type=\"html\" はエスケープ済み HTML を想定する
        writer
            .write_event(Event::Text(BytesText::new(&entry.summary_html)))
            .map_err(|e| AppError::feed(format!("summary 書き込み失敗: {}", e)))?;
        writer
            .write_event(Event::End(BytesEnd::new("summary")))
            .map_err(|e| AppError::feed(format!("summary 終了失敗: {}", e)))?;

        writer
            .write_event(Event::End(BytesEnd::new("entry")))
            .map_err(|e| AppError::feed(format!("entry 終了失敗: {}", e)))?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("feed")))
        .map_err(|e| AppError::feed(format!("feed 終了失敗: {}", e)))?;

    let output = writer
        .into_inner()
        .into_inner();
    let xml = String::from_utf8(output)
        .map_err(|e| AppError::feed(format!("XML 変換失敗: {}", e)))?;
    Ok(xml)
}

pub fn default_feed_updated(entries: &[FeedEntry], now: DateTime<Utc>) -> DateTime<FixedOffset> {
    entries
        .iter()
        .map(|e| e.updated)
        .max()
        .unwrap_or_else(|| now.with_timezone(&FixedOffset::east_opt(0).unwrap()))
}

fn write_text_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    value: &str,
) -> Result<(), AppError> {
    writer
        .write_event(Event::Start(BytesStart::new(name)))
        .map_err(|e| AppError::feed(format!("{} 開始失敗: {}", name, e)))?;
    writer
        .write_event(Event::Text(BytesText::new(value)))
        .map_err(|e| AppError::feed(format!("{} 書き込み失敗: {}", name, e)))?;
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|e| AppError::feed(format!("{} 終了失敗: {}", name, e)))?;
    Ok(())
}

fn write_link(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    rel: &str,
    href: &str,
) -> Result<(), AppError> {
    let mut link = BytesStart::new("link");
    link.push_attribute(("rel", rel));
    link.push_attribute(("href", href));
    writer
        .write_event(Event::Empty(link))
        .map_err(|e| AppError::feed(format!("link 書き込み失敗: {}", e)))?;
    Ok(())
}
