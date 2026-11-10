// src/stats.rs
// Upload date and hits of each tag, kept in SQLite.
//
// Serving a file must stay fast, so a hit is only counted in memory (a HashMap behind a Mutex).
// A background task writes all pending hits in one transaction every FLUSH_INTERVAL,
// and once more when the server stops. Losing a few seconds of hits on a crash is fine:
// they are only used for stats and cleanup, which work in days.
//
// rusqlite is synchronous, so every database access runs in spawn_blocking
// to never block the async threads that answer requests.
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rusqlite::{params, Connection, OptionalExtension};
use rocket::serde::Serialize;
use crate::config::Config;

pub const FLUSH_INTERVAL: Duration = Duration::from_secs(60);

// A tag is identified by its host ("" when STORAGE_PER_HOST is disabled) and its name
type TagKey = (String, String);

#[derive(Default)]
struct PendingHits {
    count: i64,
    last_hit_at: i64,
}

// Dates are Unix timestamps in seconds
#[derive(Serialize, Debug, PartialEq)]
pub struct TagStats {
    pub tag: String,
    // None only for a tag copied by hand and never served or uploaded since
    pub uploaded_at: Option<i64>,
    pub last_hit_at: Option<i64>,
    pub hits: i64,
}

struct Inner {
    conn: Mutex<Connection>,
    pending: Mutex<HashMap<TagKey, PendingHits>>,
}

// Cheap to clone: all clones share the same connection and counters
#[derive(Clone)]
pub struct Stats {
    inner: Arc<Inner>,
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Stats {
    pub fn open(db_path: &str) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("Cannot open database {}: {}", db_path, e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tags (
                host TEXT NOT NULL,
                tag TEXT NOT NULL,
                uploaded_at INTEGER NOT NULL,
                last_hit_at INTEGER,
                hits INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (host, tag)
            );"
        ).map_err(|e| format!("Cannot create database schema: {}", e))?;

        Ok(Stats {
            inner: Arc::new(Inner {
                conn: Mutex::new(conn),
                pending: Mutex::new(HashMap::new()),
            }),
        })
    }

    // Called on every served file: memory only, no I/O
    pub fn record_hit(&self, host: &str, tag: &str) {
        if let Ok(mut pending) = self.inner.pending.lock() {
            let hits = pending.entry((host.to_string(), tag.to_string())).or_default();
            hits.count += 1;
            hits.last_hit_at = now();
        }
    }

    // Keeps the first upload date: uploading more files to an existing tag doesn't make it new
    pub async fn record_upload(&self, host: &str, tag: &str) {
        let (host, tag) = (host.to_string(), tag.to_string());
        self.run_blocking("record upload", move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO tags (host, tag, uploaded_at) VALUES (?1, ?2, ?3)",
                params![host, tag, now()],
            ).map(|_| ())
        }).await;
    }

    pub async fn remove_tag(&self, host: &str, tag: &str) {
        let key = (host.to_string(), tag.to_string());
        if let Ok(mut pending) = self.inner.pending.lock() {
            pending.remove(&key);
        }
        self.run_blocking("remove tag", move |conn| {
            conn.execute("DELETE FROM tags WHERE host = ?1 AND tag = ?2", params![key.0, key.1]).map(|_| ())
        }).await;
    }

    // Stats of a tag, including the hits not flushed yet so the numbers are always live
    pub async fn tag_stats(&self, host: &str, tag: &str) -> TagStats {
        let key = (host.to_string(), tag.to_string());
        let (pending_count, pending_last_hit) = match self.inner.pending.lock() {
            Ok(pending) => pending.get(&key).map(|h| (h.count, Some(h.last_hit_at))).unwrap_or((0, None)),
            Err(_) => (0, None),
        };

        let stats = self.clone();
        let (host, tag) = key;
        let tag_name = tag.clone();
        let row = rocket::tokio::task::spawn_blocking(move || {
            stats.with_conn(|conn| {
                conn.query_row(
                    "SELECT uploaded_at, last_hit_at, hits FROM tags WHERE host = ?1 AND tag = ?2",
                    params![host, tag],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, i64>(2)?)),
                ).optional()
            })
        }).await;

        let (uploaded_at, last_hit_at, hits) = match row {
            Ok(Ok(Some((uploaded_at, last_hit_at, hits)))) => (Some(uploaded_at), last_hit_at, hits),
            Ok(Ok(None)) => (None, None, 0),
            Ok(Err(e)) => {
                log::error!("Stats database error (read tag stats): {}", e);
                (None, None, 0)
            }
            Err(e) => {
                log::error!("Stats task failed (read tag stats): {}", e);
                (None, None, 0)
            }
        };

        TagStats {
            tag: tag_name,
            uploaded_at,
            last_hit_at: last_hit_at.max(pending_last_hit),
            hits: hits + pending_count,
        }
    }

    pub async fn flush(&self) {
        let stats = self.clone();
        let _ = rocket::tokio::task::spawn_blocking(move || stats.flush_blocking()).await;
    }

    // Writes all pending hits in a single transaction
    pub fn flush_blocking(&self) {
        let pending = match self.inner.pending.lock() {
            Ok(mut pending) => std::mem::take(&mut *pending),
            Err(_) => return,
        };
        if pending.is_empty() {
            return;
        }

        let result = self.with_conn(|conn| {
            let tx = conn.transaction()?;
            for ((host, tag), hits) in &pending {
                // A hit on a tag missing from the database (files copied by hand) creates it
                tx.execute(
                    "INSERT INTO tags (host, tag, uploaded_at, last_hit_at, hits) VALUES (?1, ?2, ?3, ?3, ?4)
                     ON CONFLICT (host, tag) DO UPDATE SET
                        hits = hits + excluded.hits,
                        last_hit_at = MAX(COALESCE(last_hit_at, 0), excluded.last_hit_at)",
                    params![host, tag, hits.last_hit_at, hits.count],
                )?;
            }
            tx.commit()
        });

        match result {
            Ok(()) => log::debug!("Stats flushed: {} tags", pending.len()),
            Err(e) => log::error!("Failed to flush stats: {}", e),
        }
    }

    // Tags already in the storage when the database is created get the date of their
    // directory as upload date. File dates are unreliable (lost on copy), so this is
    // only done once: tags already in the database are left untouched
    pub fn seed_existing_tags(&self, config: &Config) {
        let hosts: Vec<String> = if config.storage_per_host {
            config.allowed_hosts.clone()
        } else {
            vec![String::new()]
        };

        let mut found = Vec::new();
        for host in hosts {
            let dir = Path::new(&config.storage_path).join(&host);
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };

            for entry in entries.flatten() {
                let Ok(metadata) = entry.metadata() else { continue };
                let Ok(tag) = entry.file_name().into_string() else { continue };
                if !metadata.is_dir() || crate::storage::validate_tag(&tag).is_err() {
                    continue;
                }
                let date = metadata.created().or_else(|_| metadata.modified())
                    .ok()
                    .and_then(|d| d.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or_else(now);
                found.push((host.clone(), tag, date));
            }
        }

        let result = self.with_conn(|conn| {
            let tx = conn.transaction()?;
            let mut added = 0;
            for (host, tag, date) in &found {
                added += tx.execute(
                    "INSERT OR IGNORE INTO tags (host, tag, uploaded_at) VALUES (?1, ?2, ?3)",
                    params![host, tag, date],
                )?;
            }
            tx.commit()?;
            Ok(added)
        });

        match result {
            Ok(0) => {}
            Ok(added) => log::info!("{} existing tag(s) added to the stats database", added),
            Err(e) => log::error!("Failed to add existing tags to the stats database: {}", e),
        }
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&mut Connection) -> rusqlite::Result<T>) -> rusqlite::Result<T> {
        let mut conn = self.inner.conn.lock()
            .map_err(|_| rusqlite::Error::InvalidParameterName("database mutex poisoned".to_string()))?;
        f(&mut conn)
    }

    // A failed stats write is logged but never fails the request
    async fn run_blocking<F>(&self, action: &'static str, f: F)
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<()> + Send + 'static,
    {
        let stats = self.clone();
        let result = rocket::tokio::task::spawn_blocking(move || stats.with_conn(f)).await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::error!("Stats database error ({}): {}", action, e),
            Err(e) => log::error!("Stats task failed ({}): {}", action, e),
        }
    }
}
