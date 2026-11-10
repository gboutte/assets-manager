use assets_manager::cleanup::{matches_pattern, select_tags_to_delete, TagInfo};
use assets_manager::config::CleanupConfig;

const DAY: i64 = 24 * 60 * 60;
const NOW: i64 = 1_000 * DAY;

// A tag uploaded `age` days ago, last served `idle` days ago (None: never served)
fn tag(name: &str, age: i64, idle: Option<i64>) -> TagInfo {
    let uploaded_at = NOW - age * DAY;
    TagInfo {
        tag: name.to_string(),
        uploaded_at,
        last_activity: idle.map(|d| NOW - d * DAY).unwrap_or(uploaded_at),
    }
}

fn rules() -> CleanupConfig {
    CleanupConfig::default()
}

#[test]
fn no_rule_deletes_nothing() {
    let tags = vec![tag("old", 1000, None)];
    // keep_tags or keep_last alone are not deletion rules
    let rules = CleanupConfig { keep_last: 1, keep_tags: vec!["x".to_string()], ..rules() };
    assert!(select_tags_to_delete(&tags, &rules, NOW).is_empty());
}

#[test]
fn max_age() {
    let tags = vec![tag("new", 5, Some(0)), tag("limit", 30, None), tag("old", 40, Some(0))];
    let rules = CleanupConfig { max_age_days: Some(30), ..rules() };
    // Hits don't save a tag that is too old
    assert_eq!(select_tags_to_delete(&tags, &rules, NOW), vec!["limit", "old"]);
}

#[test]
fn max_idle() {
    let tags = vec![
        tag("served-recently", 100, Some(2)),
        tag("not-served-for-long", 100, Some(20)),
        tag("never-served-new", 3, None),
        tag("never-served-old", 15, None),
    ];
    let rules = CleanupConfig { max_idle_days: Some(10), ..rules() };
    // A tag never served is idle since its upload
    assert_eq!(select_tags_to_delete(&tags, &rules, NOW), vec!["never-served-old", "not-served-for-long"]);
}

#[test]
fn age_or_idle() {
    let tags = vec![tag("old-but-used", 50, Some(1)), tag("new-but-unused", 5, Some(20)), tag("fine", 5, Some(1))];
    let rules = CleanupConfig { max_age_days: Some(30), max_idle_days: Some(10), ..rules() };
    assert_eq!(select_tags_to_delete(&tags, &rules, NOW), vec!["new-but-unused", "old-but-used"]);
}

#[test]
fn keep_last_only() {
    // max_age 0 makes every tag eligible: only the keep_last most recent remain
    let tags = vec![tag("v1", 40, None), tag("v2", 30, None), tag("v3", 20, None), tag("v4", 10, None)];
    let rules = CleanupConfig { max_age_days: Some(0), keep_last: 2, ..rules() };
    assert_eq!(select_tags_to_delete(&tags, &rules, NOW), vec!["v1", "v2"]);
}

#[test]
fn idle_but_keep_last_versions() {
    // "Delete after 30 days without hits, but always keep the 3 last versions"
    let tags = vec![
        tag("v1", 100, None), tag("v2", 90, None), tag("v3", 80, None),
        tag("v4", 70, None), tag("v5", 60, Some(1)),
    ];
    let rules = CleanupConfig { max_idle_days: Some(30), keep_last: 3, ..rules() };
    // v3, v4, v5 are the 3 most recent uploads; v5 is also still used
    assert_eq!(select_tags_to_delete(&tags, &rules, NOW), vec!["v1", "v2"]);
}

#[test]
fn keep_last_bigger_than_tag_count() {
    let tags = vec![tag("v1", 100, None), tag("v2", 90, None)];
    let rules = CleanupConfig { max_age_days: Some(1), keep_last: 10, ..rules() };
    assert!(select_tags_to_delete(&tags, &rules, NOW).is_empty());
}

#[test]
fn keep_tags() {
    let tags = vec![tag("prod", 100, None), tag("v1.0.0", 100, None), tag("v1.2", 100, None), tag("v2.0.0", 100, None)];
    let rules = CleanupConfig {
        max_age_days: Some(30),
        keep_tags: vec!["prod".to_string(), "v1.*".to_string()],
        ..rules()
    };
    assert_eq!(select_tags_to_delete(&tags, &rules, NOW), vec!["v2.0.0"]);
}

#[test]
fn keep_tags_and_keep_last_add_up() {
    let tags = vec![tag("prod", 100, None), tag("v1", 50, None), tag("v2", 40, None), tag("v3", 30, None)];
    let rules = CleanupConfig {
        max_age_days: Some(0),
        keep_last: 1,
        keep_tags: vec!["prod".to_string()],
        ..rules()
    };
    // prod is pinned, v3 is the most recent upload
    assert_eq!(select_tags_to_delete(&tags, &rules, NOW), vec!["v1", "v2"]);
}

#[test]
fn pattern_matching() {
    assert!(matches_pattern("prod", "prod"));
    assert!(!matches_pattern("prod", "prod2"));
    assert!(matches_pattern("v1.*", "v1.0.0"));
    assert!(matches_pattern("v1.*", "v1."));
    assert!(!matches_pattern("v1.*", "v10.0"));
    assert!(matches_pattern("*-stable", "2026-stable"));
    assert!(!matches_pattern("*-stable", "stable"));
    assert!(matches_pattern("release-*-*", "release-2026-11"));
    assert!(!matches_pattern("release-*-*", "release-2026"));
    assert!(matches_pattern("*", "anything"));
    // The prefix and suffix must not overlap
    assert!(!matches_pattern("ab*ba", "aba"));
}

// --- run_cleanup on a real storage and database ---

use std::path::Path;
use rocket::local::blocking::Client;
use rusqlite::{params, Connection};
use serial_test::serial;
use assets_manager::cleanup::{run_cleanup, DeletedTag};
use assets_manager::config::Config;
use assets_manager::create_rocket;
use assets_manager::stats::{now, Stats};

const TEST_DB: &str = "./upload/.cleanup-test.db";

fn test_config(cleanup: CleanupConfig) -> Config {
    Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
        db_path: TEST_DB.to_string(),
        cleanup,
    }
}

fn cleanup_storage() {
    let _ = std::fs::remove_file(TEST_DB);
    for entry in std::fs::read_dir("./upload").unwrap().flatten() {
        if entry.path().is_dir() {
            std::fs::remove_dir_all(entry.path()).unwrap();
        }
    }
}

// Creates the tag on disk and sets its dates in the database
fn add_tag(host: &str, tag: &str, age_days: i64) {
    let dir = Path::new("./upload").join(host).join(tag);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("f.txt"), "content").unwrap();
    Connection::open(TEST_DB).unwrap().execute(
        "INSERT OR REPLACE INTO tags (host, tag, uploaded_at) VALUES (?1, ?2, ?3)",
        params![host, tag, now() - age_days * DAY],
    ).unwrap();
}

fn tag_in_db(host: &str, tag: &str) -> bool {
    Connection::open(TEST_DB).unwrap()
        .query_row("SELECT COUNT(*) FROM tags WHERE host = ?1 AND tag = ?2", params![host, tag], |r| r.get::<_, i64>(0))
        .unwrap() == 1
}

fn run(client: &Client, dry_run: bool) -> Vec<DeletedTag> {
    let config = client.rocket().state::<Config>().unwrap();
    let stats = client.rocket().state::<Stats>().unwrap();
    rocket::execute(run_cleanup(config, stats, dry_run))
}

fn deleted(tags: &[&str]) -> Vec<DeletedTag> {
    tags.iter().map(|t| DeletedTag { host: String::new(), tag: t.to_string() }).collect()
}

#[test]
#[serial]
fn cleanup_dry_run_then_real() {
    cleanup_storage();
    let rules = CleanupConfig { max_age_days: Some(30), keep_tags: vec!["prod".to_string()], ..CleanupConfig::default() };
    let client = Client::tracked(create_rocket(test_config(rules))).unwrap();
    add_tag("", "old", 40);
    add_tag("", "prod", 40);
    add_tag("", "new", 5);

    // Dry run: reports but keeps everything
    assert_eq!(run(&client, true), deleted(&["old"]));
    assert!(Path::new("./upload/old").exists());
    assert!(tag_in_db("", "old"));

    assert_eq!(run(&client, false), deleted(&["old"]));
    assert!(!Path::new("./upload/old").exists());
    assert!(!tag_in_db("", "old"));
    assert!(Path::new("./upload/prod").exists());
    assert!(Path::new("./upload/new").exists());

    // Nothing left to delete
    assert!(run(&client, false).is_empty());

    cleanup_storage();
}

#[test]
#[serial]
fn cleanup_uses_pending_hits() {
    cleanup_storage();
    let rules = CleanupConfig { max_idle_days: Some(10), ..CleanupConfig::default() };
    let client = Client::tracked(create_rocket(test_config(rules))).unwrap();
    add_tag("", "unused", 20);
    add_tag("", "used", 20);

    // Served just now, hit still in memory: run_cleanup flushes before reading the dates
    assert_eq!(client.get("/used/f.txt").dispatch().status(), rocket::http::Status::Ok);
    assert_eq!(run(&client, false), deleted(&["unused"]));
    assert!(Path::new("./upload/used").exists());

    cleanup_storage();
}

#[test]
#[serial]
fn cleanup_never_deletes_unknown_tags() {
    cleanup_storage();
    let rules = CleanupConfig { max_age_days: Some(0), ..CleanupConfig::default() };
    let client = Client::tracked(create_rocket(test_config(rules))).unwrap();

    // Copied by hand after startup: no date in the database.
    // Even with max_age 0 (every dated tag is eligible), it is kept on this run
    std::fs::create_dir_all("./upload/copied").unwrap();
    assert!(run(&client, false).is_empty());
    assert!(Path::new("./upload/copied").exists());

    // It now has a date and ages like any other tag
    assert!(tag_in_db("", "copied"));
    assert_eq!(run(&client, false), deleted(&["copied"]));

    cleanup_storage();
}

#[test]
#[serial]
fn cleanup_without_rules_does_nothing() {
    cleanup_storage();
    let client = Client::tracked(create_rocket(test_config(CleanupConfig::default()))).unwrap();
    add_tag("", "very-old", 1000);

    assert!(run(&client, false).is_empty());
    assert!(Path::new("./upload/very-old").exists());

    cleanup_storage();
}

#[test]
#[serial]
fn cleanup_per_host() {
    cleanup_storage();
    let rules = CleanupConfig { max_age_days: Some(0), keep_last: 1, ..CleanupConfig::default() };
    let config = Config {
        storage_per_host: true,
        allowed_hosts: vec!["assets.domain1.com".to_string(), "assets.domain2.com".to_string()],
        ..test_config(rules)
    };
    let client = Client::tracked(create_rocket(config)).unwrap();
    add_tag("assets.domain1.com", "v1", 20);
    add_tag("assets.domain1.com", "v2", 10);
    add_tag("assets.domain2.com", "v1", 20);

    // keep_last applies per domain: domain2 keeps its only tag
    let result = run(&client, false);
    assert_eq!(result, vec![DeletedTag { host: "assets.domain1.com".to_string(), tag: "v1".to_string() }]);
    assert!(!Path::new("./upload/assets.domain1.com/v1").exists());
    assert!(Path::new("./upload/assets.domain1.com/v2").exists());
    assert!(Path::new("./upload/assets.domain2.com/v1").exists());

    cleanup_storage();
}
