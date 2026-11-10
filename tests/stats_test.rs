use std::path::Path;
use rocket::http::{ContentType, Header, Status};
use rocket::local::blocking::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serial_test::serial;
use assets_manager::config::Config;
use assets_manager::create_rocket;
use assets_manager::stats::{now, Stats};

const STORAGE: &str = "./upload";
const TEST_DB: &str = "./upload/.stats-test.db";

fn test_config() -> Config {
    Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: STORAGE.to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
        db_path: TEST_DB.to_string(),
    }
}

fn cleanup() {
    let _ = std::fs::remove_file(TEST_DB);
    for entry in std::fs::read_dir(STORAGE).unwrap().flatten() {
        if entry.path().is_dir() {
            std::fs::remove_dir_all(entry.path()).unwrap();
        }
    }
}

// (uploaded_at, last_hit_at, hits) of a tag, read straight from the database
fn tag_row(host: &str, tag: &str) -> Option<(i64, Option<i64>, i64)> {
    let conn = Connection::open(TEST_DB).unwrap();
    conn.query_row(
        "SELECT uploaded_at, last_hit_at, hits FROM tags WHERE host = ?1 AND tag = ?2",
        params![host, tag],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().unwrap()
}

fn upload(client: &Client, host: Option<&str>, path: &str) -> Status {
    let boundary = "----TestBoundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"f.txt\"\r\n\
         Content-Type: application/octet-stream\r\n\r\nhello\r\n--{boundary}--\r\n"
    );
    let mut request = client.post(format!("/upload/{}", path))
        .header(ContentType::new("multipart", "form-data").with_params([("boundary", boundary)]))
        .header(Header::new("Authorization", "Bearer test-token"))
        .body(body);
    if let Some(host) = host {
        request = request.header(Header::new("Host", host.to_string()));
    }
    request.dispatch().status()
}

fn flush(client: &Client) {
    client.rocket().state::<Stats>().expect("stats are managed").flush_blocking();
}

#[test]
#[serial]
fn upload_records_date() {
    cleanup();
    let client = Client::tracked(create_rocket(test_config())).unwrap();

    let before = now();
    assert_eq!(upload(&client, None, "v1.0.0/f.txt"), Status::Ok);

    let (uploaded_at, last_hit_at, hits) = tag_row("", "v1.0.0").expect("tag recorded");
    assert!(uploaded_at >= before && uploaded_at <= now());
    assert_eq!(last_hit_at, None);
    assert_eq!(hits, 0);

    cleanup();
}

#[test]
#[serial]
fn upload_keeps_first_date() {
    cleanup();
    let client = Client::tracked(create_rocket(test_config())).unwrap();

    assert_eq!(upload(&client, None, "v1.0.0/f.txt"), Status::Ok);
    Connection::open(TEST_DB).unwrap()
        .execute("UPDATE tags SET uploaded_at = 1000 WHERE tag = 'v1.0.0'", []).unwrap();

    // More files in the same tag: still the same upload date
    assert_eq!(upload(&client, None, "v1.0.0/other.txt"), Status::Ok);
    assert_eq!(tag_row("", "v1.0.0").unwrap().0, 1000);

    cleanup();
}

#[test]
#[serial]
fn hits_are_written_on_flush() {
    cleanup();
    let client = Client::tracked(create_rocket(test_config())).unwrap();
    assert_eq!(upload(&client, None, "v1.0.0/f.txt"), Status::Ok);

    for _ in 0..3 {
        assert_eq!(client.get("/v1.0.0/f.txt").dispatch().status(), Status::Ok);
    }
    // A missing file is not a hit
    assert_eq!(client.get("/v1.0.0/missing.txt").dispatch().status(), Status::NotFound);

    // Hits are only counted in memory until the flush
    assert_eq!(tag_row("", "v1.0.0").unwrap().2, 0);

    let before = now();
    flush(&client);
    let (_, last_hit_at, hits) = tag_row("", "v1.0.0").unwrap();
    assert_eq!(hits, 3);
    assert!(last_hit_at.unwrap() >= before - 1);

    // Next flush adds to the previous count
    client.get("/v1.0.0/f.txt").dispatch();
    flush(&client);
    assert_eq!(tag_row("", "v1.0.0").unwrap().2, 4);

    cleanup();
}

#[test]
#[serial]
fn delete_removes_stats() {
    cleanup();
    let client = Client::tracked(create_rocket(test_config())).unwrap();
    assert_eq!(upload(&client, None, "v1.0.0/f.txt"), Status::Ok);
    client.get("/v1.0.0/f.txt").dispatch();

    let response = client.delete("/tags/v1.0.0").header(Header::new("Authorization", "Bearer test-token")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    flush(&client);

    // Pending hits of the deleted tag are dropped too
    assert_eq!(tag_row("", "v1.0.0"), None);

    cleanup();
}

#[test]
#[serial]
fn stats_per_host() {
    cleanup();
    let config = Config {
        storage_per_host: true,
        allowed_hosts: vec!["assets.domain1.com".to_string(), "assets.domain2.com".to_string()],
        ..test_config()
    };
    let client = Client::tracked(create_rocket(config)).unwrap();

    assert_eq!(upload(&client, Some("assets.domain1.com"), "v1.0.0/f.txt"), Status::Ok);
    assert_eq!(upload(&client, Some("assets.domain2.com"), "v1.0.0/f.txt"), Status::Ok);
    client.get("/v1.0.0/f.txt").header(Header::new("Host", "assets.domain1.com")).dispatch();
    flush(&client);

    assert_eq!(tag_row("assets.domain1.com", "v1.0.0").unwrap().2, 1);
    assert_eq!(tag_row("assets.domain2.com", "v1.0.0").unwrap().2, 0);
    assert_eq!(tag_row("", "v1.0.0"), None);

    cleanup();
}

#[test]
#[serial]
fn existing_tags_are_seeded() {
    cleanup();
    std::fs::create_dir_all("./upload/old-tag").unwrap();
    std::fs::write("./upload/old-tag/f.txt", "old").unwrap();
    // Not a valid tag: ignored
    std::fs::create_dir_all("./upload/.hidden").unwrap();

    let client = Client::tracked(create_rocket(test_config())).unwrap();
    assert!(tag_row("", "old-tag").is_some());
    assert!(tag_row("", ".hidden").is_none());

    // Seeding only fills missing tags: an existing date is kept on restart
    Connection::open(TEST_DB).unwrap()
        .execute("UPDATE tags SET uploaded_at = 1000 WHERE tag = 'old-tag'", []).unwrap();
    drop(client);
    let _client = Client::tracked(create_rocket(test_config())).unwrap();
    assert_eq!(tag_row("", "old-tag").unwrap().0, 1000);

    cleanup();
}

#[test]
#[serial]
fn hit_on_tag_missing_from_database() {
    cleanup();
    let client = Client::tracked(create_rocket(test_config())).unwrap();

    // Files copied by hand after startup
    std::fs::create_dir_all("./upload/copied").unwrap();
    std::fs::write("./upload/copied/f.txt", "copied").unwrap();

    assert_eq!(client.get("/copied/f.txt").dispatch().status(), Status::Ok);
    flush(&client);
    assert_eq!(tag_row("", "copied").unwrap().2, 1);

    cleanup();
}

#[test]
#[serial]
fn invalid_database_path_prevents_startup() {
    cleanup();
    let config = Config { db_path: "./upload/missing-dir/stats.db".to_string(), ..test_config() };

    // Reading the error kind marks it as handled (Rocket panics when an unhandled error is dropped)
    let error = Client::tracked(create_rocket(config)).err().expect("startup must fail");
    assert!(matches!(error.kind(), rocket::error::ErrorKind::FailedFairings(_)));
    assert!(!Path::new("./upload/missing-dir").exists());
}
