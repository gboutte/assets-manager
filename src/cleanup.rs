// src/cleanup.rs
// Automatic deletion of old or unused tags.
//
// The rules are a pure function (select_tags_to_delete): no disk, no clock, so every
// combination can be unit tested. run_cleanup applies them to the storage.
use std::collections::HashMap;
use std::path::Path;
use rocket::serde::Serialize;
use crate::config::{CleanupConfig, Config};
use crate::stats::{now, Stats};
use crate::storage::validate_tag;

const DAY: i64 = 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct TagInfo {
    pub tag: String,
    pub uploaded_at: i64,
    // Last hit, or the upload date for a tag never served: a new tag isn't idle
    pub last_activity: i64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct DeletedTag {
    // "" when STORAGE_PER_HOST is disabled
    pub host: String,
    pub tag: String,
}

// '*' matches any sequence of characters: "v1.*" matches "v1.0.0" and "v1.2"
pub fn matches_pattern(pattern: &str, tag: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == tag;
    }

    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !tag.starts_with(first) || !tag[first.len()..].ends_with(last) || tag.len() < first.len() + last.len() {
        return false;
    }

    // Middle parts must appear in order between the prefix and the suffix
    let mut rest = &tag[first.len()..tag.len() - last.len()];
    for part in &parts[1..parts.len() - 1] {
        match rest.find(part) {
            Some(index) => rest = &rest[index + part.len()..],
            None => return false,
        }
    }
    true
}

pub fn select_tags_to_delete(tags: &[TagInfo], rules: &CleanupConfig, now: i64) -> Vec<String> {
    if !rules.has_rules() {
        return Vec::new();
    }

    // The keep_last most recent uploads are protected
    let mut by_upload: Vec<&TagInfo> = tags.iter().collect();
    by_upload.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at).then_with(|| a.tag.cmp(&b.tag)));
    let recent: Vec<&str> = by_upload.iter().take(rules.keep_last).map(|t| t.tag.as_str()).collect();

    let too_old = |t: &TagInfo| rules.max_age_days
        .is_some_and(|days| now - t.uploaded_at >= days as i64 * DAY);
    let idle = |t: &TagInfo| rules.max_idle_days
        .is_some_and(|days| now - t.last_activity >= days as i64 * DAY);
    let protected = |t: &TagInfo| recent.contains(&t.tag.as_str())
        || rules.keep_tags.iter().any(|pattern| matches_pattern(pattern, &t.tag));

    let mut selected: Vec<String> = tags.iter()
        .filter(|t| (too_old(t) || idle(t)) && !protected(t))
        .map(|t| t.tag.clone())
        .collect();
    selected.sort();
    selected
}

// Applies the rules to every storage directory (one per host when STORAGE_PER_HOST is enabled).
// With dry_run, nothing is deleted: the returned list is what would be deleted
pub async fn run_cleanup(config: &Config, stats: &Stats, dry_run: bool) -> Vec<DeletedTag> {
    let rules = &config.cleanup;
    if !rules.has_rules() {
        return Vec::new();
    }

    // The rules need up to date hits
    stats.flush().await;

    let hosts: Vec<String> = if config.storage_per_host {
        config.allowed_hosts.clone()
    } else {
        vec![String::new()]
    };

    let now = now();
    let mut deleted = Vec::new();

    for host in hosts {
        let dir = Path::new(&config.storage_path).join(&host);
        let known = stats.tag_dates(&host).await;
        let (tags, unknown) = list_tags(&dir, &known);

        // Tags with no date (copied by hand after startup) are never deleted on this run:
        // they get today as upload date and age normally from now on
        for tag in unknown {
            stats.record_upload(&host, &tag).await;
        }

        for tag in select_tags_to_delete(&tags, rules, now) {
            if dry_run {
                log::info!("Cleanup (dry run): would delete tag {} {}", tag, host_label(&host));
            } else {
                if let Err(e) = rocket::tokio::fs::remove_dir_all(dir.join(&tag)).await {
                    log::error!("Cleanup: failed to delete tag {} {}: {}", tag, host_label(&host), e);
                    continue;
                }
                stats.remove_tag(&host, &tag).await;
                log::info!("Cleanup: deleted tag {} {}", tag, host_label(&host));
            }
            deleted.push(DeletedTag { host: host.clone(), tag });
        }
    }

    deleted
}

fn host_label(host: &str) -> String {
    if host.is_empty() { String::new() } else { format!("on {}", host) }
}

// Tags present on disk with their dates, and the tags missing from the database
fn list_tags(dir: &Path, known: &HashMap<String, (i64, Option<i64>)>) -> (Vec<TagInfo>, Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return (Vec::new(), Vec::new()) };

    let mut tags = Vec::new();
    let mut unknown = Vec::new();
    for entry in entries.flatten() {
        let Ok(tag) = entry.file_name().into_string() else { continue };
        if !entry.path().is_dir() || validate_tag(&tag).is_err() {
            continue;
        }
        match known.get(&tag) {
            Some(&(uploaded_at, last_hit_at)) => tags.push(TagInfo {
                last_activity: last_hit_at.unwrap_or(uploaded_at).max(uploaded_at),
                uploaded_at,
                tag,
            }),
            None => unknown.push(tag),
        }
    }
    (tags, unknown)
}
