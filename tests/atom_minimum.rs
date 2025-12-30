use chrono::{DateTime, FixedOffset, Utc};
use dev_to_high_reactions_rss::atom::{build_feed_xml, default_feed_updated, FeedEntry, FeedInfo};

#[test]
fn atom_minimum_requirements() {
    let updated: DateTime<FixedOffset> =
        DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap();
    let entry = FeedEntry {
        id: "tag:dev.to,2024:1".to_string(),
        title: "Test".to_string(),
        link: "https://dev.to/test".to_string(),
        updated,
        summary_html: "Reactions: 1".to_string(),
    };

    let now = Utc::now();
    let feed = FeedInfo {
        id: "https://example.com/feed.xml".to_string(),
        title: "Sample".to_string(),
        description: "Desc".to_string(),
        updated: default_feed_updated(&[entry.clone()], now),
        feed_url: "https://example.com/feed.xml".to_string(),
        index_url: "https://example.com/index.html".to_string(),
        entries: vec![entry],
    };

    let xml = build_feed_xml(&feed).expect("feed xml");
    let doc = roxmltree::Document::parse(&xml).expect("xml parse");

    let feed_node = doc.descendants().find(|n| n.has_tag_name("feed")).unwrap();
    assert!(
        feed_node
            .children()
            .any(|n| n.has_tag_name("id") && n.text().is_some()),
        "feed id missing"
    );
    assert!(
        feed_node
            .children()
            .any(|n| n.has_tag_name("title") && n.text().is_some()),
        "feed title missing"
    );
    assert!(
        feed_node
            .children()
            .any(|n| n.has_tag_name("updated") && n.text().is_some()),
        "feed updated missing"
    );

    let self_link = feed_node
        .children()
        .find(|n| n.has_tag_name("link") && n.attribute("rel") == Some("self"));
    assert!(self_link.is_some(), "self link missing");

    let alt_link = feed_node
        .children()
        .find(|n| n.has_tag_name("link") && n.attribute("rel") == Some("alternate"));
    assert!(alt_link.is_some(), "alternate link missing");

    let entry_node = feed_node
        .children()
        .find(|n| n.has_tag_name("entry"))
        .expect("entry missing");

    assert!(
        entry_node
            .children()
            .any(|n| n.has_tag_name("id") && n.text().is_some()),
        "entry id missing"
    );
    assert!(
        entry_node
            .children()
            .any(|n| n.has_tag_name("title") && n.text().is_some()),
        "entry title missing"
    );
    assert!(
        entry_node
            .children()
            .any(|n| n.has_tag_name("link") && n.attribute("href").is_some()),
        "entry link missing"
    );
    assert!(
        entry_node
            .children()
            .any(|n| n.has_tag_name("updated") && n.text().is_some()),
        "entry updated missing"
    );
    let summary = entry_node
        .children()
        .find(|n| n.has_tag_name("summary"))
        .expect("summary missing");
    assert_eq!(summary.attribute("type"), Some("html"));
}
