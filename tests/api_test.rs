use std::path::Path;
use rocket::http::{ContentType, Header, Status};
use rocket::local::blocking::{Client};
use assets_manager::config::Config;
use assets_manager::create_rocket;
use rocket::serde::Deserialize;
use serial_test::serial;

use std::io::{Write, Cursor};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Deserialize)]
struct HealthResult {
    status: String,
}

#[test]
#[serial]
fn health_check() {

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");
    let response = client.get("/health").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let result = response.into_json::<HealthResult>();
    assert!(result.is_some());
    assert_eq!(result.unwrap().status, "ok");
}
#[test]
#[serial]
fn health_check_degraded() {

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./test-uploads".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");
    let response = client.get("/health").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let result = response.into_json::<HealthResult>();
    assert!(result.is_some());
    assert_eq!(result.unwrap().status, "degraded");
}


#[derive(Deserialize)]
struct TagsList {
    tags: Vec<String>,
}
#[test]
#[serial]
fn tags_list() {

    cleanup_upload_dirs();
    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");
    let response = client.get("/tags").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let result = response.into_json::<TagsList>();
    assert!(result.is_some());
    assert_eq!(result.unwrap().tags.len(), 0);

    // We add a tag directory
    std::fs::create_dir("./upload/test-tag").unwrap();
    let response = client.get("/tags").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let result = response.into_json::<TagsList>();
    assert!(result.is_some());
    let tags_respo = result.unwrap();
    assert_eq!(tags_respo.tags.len(), 1);
    assert_eq!(tags_respo.tags[0], "test-tag");


    //Clean up
    std::fs::remove_dir("./upload/test-tag").unwrap();


}
#[test]
#[serial]
fn delete_tag() {

    cleanup_upload_dirs();
    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");

    std::fs::create_dir("./upload/test-tag").unwrap();



    let response = client.delete("/tags/test-tag").dispatch();
    assert_eq!(response.status(), Status::Unauthorized);


    assert!(Path::new("./upload/test-tag").exists());

    let response = client.delete("/tags/test-tag").header(Header::new("Authorization","Bearer test-token")).dispatch();
    assert_eq!(response.status(), Status::Ok);

    assert!(!Path::new("./upload/test-tag").exists());
}
#[test]
#[serial]
fn delete_tag_not_found() {

    cleanup_upload_dirs();
    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");

    let response = client.delete("/tags/unknown-tag").header(Header::new("Authorization","Bearer test-token")).dispatch();
    assert_eq!(response.status(), Status::NotFound);
}
#[test]
#[serial]
fn multiple_api_tokens() {

    cleanup_upload_dirs();
    let config: Config = Config {
        api_tokens: vec!["token-ci".to_string(), "token-deploy".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");

    // Authenticated requests on an unknown tag get 404, rejected ones get 401
    for (header, expected) in [
        ("Bearer token-ci", Status::NotFound),
        ("Bearer token-deploy", Status::NotFound),
        ("Bearer token-unknown", Status::Unauthorized),
        ("Bearer ", Status::Unauthorized),
        ("token-ci", Status::Unauthorized),
    ] {
        let response = client.delete("/tags/unknown-tag").header(Header::new("Authorization", header)).dispatch();
        assert_eq!(response.status(), expected, "Authorization: {}", header);
    }
}
#[test]
#[serial]
fn file_get() {


    cleanup_upload_dirs();


    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");
    let response = client.get("/v1.0.0/test-file.txt").dispatch();
    assert_eq!(response.status(), Status::NotFound,"/v1.0.0/test-file.txt should be not found");
    // An error must never be cached
    assert_eq!(response.headers().get_one("Cache-Control"), None);



    std::fs::create_dir("./upload/v1.0.0").unwrap();
    std::fs::write("./upload/v1.0.0/test-file.txt", "test").unwrap();

    let response = client.get("/v1.0.0/test-file.txt").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.headers().get_one("Cache-Control"), Some("public, max-age=31536000, immutable"));
    assert_eq!(response.into_string(), Some("test".to_string()));



    //Clean up
    cleanup_upload_dirs();

}
#[test]
#[serial]
fn file_get_cache_disabled() {

    cleanup_upload_dirs();

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 0,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");

    std::fs::create_dir("./upload/v1.0.0").unwrap();
    std::fs::write("./upload/v1.0.0/test-file.txt", "test").unwrap();

    let response = client.get("/v1.0.0/test-file.txt").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Cache-Control"), Some("no-cache"));

    //Clean up
    cleanup_upload_dirs();
}

#[test]
#[serial]
fn file_get_path_traversal() {


    cleanup_upload_dirs();


    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");
    let response = client.get("/test-tag/../../../../test-file.txt").dispatch();
    assert_eq!(response.status(), Status::NotFound,"/test-tag/../../test-file.txt should be not found");



    //Clean up
    cleanup_upload_dirs();

}

fn cleanup_upload_dirs() {
    let upload_path = Path::new("./upload");
    if !upload_path.exists() {
        return;
    }

    for entry in std::fs::read_dir(upload_path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            std::fs::remove_dir_all(entry.path()).unwrap();
        }
    }
}

#[test]
#[serial]
fn test_upload_file() {


    //Clean up
    cleanup_upload_dirs();

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).unwrap();


    let (content_type, body) = multipart_body("test-file.txt", "Hello world!");
    let auth_header = Header::new("Authorization","Bearer test-token");

    let response = client.post("/upload/v1.0.0/test-file.txt")
        .header(content_type)
        .header(auth_header)
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    //Should be the get asset uri
    assert_eq!(response.into_string(), Some("http://localhost/v1.0.0/test-file.txt".to_string()));

    assert!(Path::new("./upload/v1.0.0/test-file.txt").exists());
    //Check file content
    let file_content = std::fs::read("./upload/v1.0.0/test-file.txt").unwrap();
    assert_eq!(file_content, b"Hello world!");

    //Clean up
    cleanup_upload_dirs();

}

#[test]
#[serial]
fn test_upload_file_no_auth() {

    //Clean up
    cleanup_upload_dirs();

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).unwrap();


    let (content_type, body) = multipart_body("test-file.txt", "Hello world!");

    let response = client.post("/upload/v1.0.0/test-file.txt")
        .header(content_type)
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::Unauthorized);
    //Should be the get asset uri

    assert!(!Path::new("./upload/v1.0.0/test-file.txt").exists());

    //Clean up
    cleanup_upload_dirs();
}

#[test]
#[serial]
fn test_upload_file_persist_error() {

    //Clean up
    cleanup_upload_dirs();

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).unwrap();

    // A directory already exists where the file should be written, so the write fails
    std::fs::create_dir_all("./upload/v1.0.0/test-file.txt").unwrap();

    let (content_type, body) = multipart_body("test-file.txt", "Hello world!");
    let auth_header = Header::new("Authorization","Bearer test-token");

    let response = client.post("/upload/v1.0.0/test-file.txt")
        .header(content_type)
        .header(auth_header)
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::InternalServerError);

    //Clean up
    cleanup_upload_dirs();
}

#[test]
#[serial]
fn test_upload_zip() {


    //Clean up
    cleanup_upload_dirs();

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).unwrap();


    let auth_header = Header::new("Authorization","Bearer test-token");
    // Create a ZIP in memory or read from test fixtures
    let zip_bytes = create_test_zip(); // Helper to create ZIP with test files


    let (content_type, body) = multipart_body_binary("archive.zip", &zip_bytes, "application/zip");

    let response = client.post("/upload/v1.0.0")
        .header(content_type)
        .header(auth_header)
        .body(body)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);

    assert!(Path::new("./upload/v1.0.0/file1.txt").exists());
    assert!(Path::new("./upload/v1.0.0/file2.txt").exists());
    //Check file content
    let file_content1 = std::fs::read("./upload/v1.0.0/file1.txt").unwrap();
    assert_eq!(file_content1, b"Hello world File 1!");

    let file_content2 = std::fs::read("./upload/v1.0.0/file2.txt").unwrap();
    assert_eq!(file_content2, b"Hello world File 2!");

    //Clean up
    cleanup_upload_dirs();

}

#[test]
#[serial]
fn test_upload_zip_slip() {

    // Files the malicious zip tries to write outside of ./upload/v1.0.0
    let escaped_files = ["./upload/zip-slip-evil.txt", "./zip-slip-evil.txt"];

    //Clean up
    cleanup_upload_dirs();
    for file in escaped_files {
        let _ = std::fs::remove_file(file);
    }

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).unwrap();


    let auth_header = Header::new("Authorization","Bearer test-token");
    let zip_bytes = create_zip_slip_zip();

    let (content_type, body) = multipart_body_binary("archive.zip", &zip_bytes, "application/zip");

    let response = client.post("/upload/v1.0.0")
        .header(content_type)
        .header(auth_header)
        .body(body)
        .dispatch();

    // The archive is invalid because of the client, not because of the server
    assert_eq!(response.status(), Status::BadRequest);

    // No file must have been written outside of the tag directory
    for file in escaped_files {
        assert!(!Path::new(file).exists(), "{} should not exist", file);
    }

    //Clean up
    cleanup_upload_dirs();
    for file in escaped_files {
        let _ = std::fs::remove_file(file);
    }

}


#[test]
#[serial]
fn invalid_tag() {

    cleanup_upload_dirs();

    let config: Config = Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: false,
        allowed_hosts: vec![],
    };

    let client = Client::tracked(create_rocket(config)).unwrap();
    let auth_header = Header::new("Authorization","Bearer test-token");

    // Hidden directory created by hand, must not be reachable through the API
    std::fs::create_dir("./upload/.hidden").unwrap();
    std::fs::write("./upload/.hidden/test-file.txt", "test").unwrap();

    let response = client.get("/.hidden/test-file.txt").dispatch();
    assert_eq!(response.status(), Status::BadRequest);

    let (content_type, body) = multipart_body("test-file.txt", "Hello world!");
    let response = client.post("/upload/v1@0/test-file.txt")
        .header(content_type)
        .header(auth_header.clone())
        .body(body)
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert!(!Path::new("./upload/v1@0").exists());

    let (content_type, body) = multipart_body_binary("archive.zip", &create_test_zip(), "application/zip");
    let response = client.post("/upload/v1@0")
        .header(content_type)
        .header(auth_header.clone())
        .body(body)
        .dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert!(!Path::new("./upload/v1@0").exists());

    let response = client.delete("/tags/.hidden").header(auth_header).dispatch();
    assert_eq!(response.status(), Status::BadRequest);
    assert!(Path::new("./upload/.hidden").exists());

    //Clean up
    cleanup_upload_dirs();

}

fn per_host_config() -> Config {
    Config {
        api_tokens: vec!["test-token".to_string()],
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
        cache_max_age: 31536000,
        storage_per_host: true,
        allowed_hosts: vec!["assets.domain1.com".to_string(), "assets.domain2.com".to_string()],
    }
}

fn upload_on_host(client: &Client, host: &str, path: &str, content: &str) -> Status {
    let (content_type, body) = multipart_body("test-file.txt", content);
    client.post(format!("/upload/{}", path))
        .header(content_type)
        .header(Header::new("Authorization","Bearer test-token"))
        .header(Header::new("Host", host.to_string()))
        .body(body)
        .dispatch()
        .status()
}

#[test]
#[serial]
fn per_host_isolation() {

    cleanup_upload_dirs();
    let client = Client::tracked(create_rocket(per_host_config())).unwrap();

    // Same tag and file name on both domains
    assert_eq!(upload_on_host(&client, "assets.domain1.com", "v1.0.0/test-file.txt", "domain1"), Status::Ok);
    assert_eq!(upload_on_host(&client, "assets.domain2.com", "v1.0.0/test-file.txt", "domain2"), Status::Ok);

    assert_eq!(std::fs::read("./upload/assets.domain1.com/v1.0.0/test-file.txt").unwrap(), b"domain1");
    assert_eq!(std::fs::read("./upload/assets.domain2.com/v1.0.0/test-file.txt").unwrap(), b"domain2");

    // Each domain serves its own file
    for (host, content) in [("assets.domain1.com", "domain1"), ("assets.domain2.com", "domain2")] {
        let response = client.get("/v1.0.0/test-file.txt").header(Header::new("Host", host)).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string(), Some(content.to_string()));
    }

    // A tag only uploaded on domain1 doesn't exist on domain2
    assert_eq!(upload_on_host(&client, "assets.domain1.com", "only-domain1/test-file.txt", "domain1"), Status::Ok);
    let response = client.get("/only-domain1/test-file.txt").header(Header::new("Host", "assets.domain2.com")).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.get("/tags").header(Header::new("Host", "assets.domain2.com")).dispatch();
    assert_eq!(response.into_json::<TagsList>().unwrap().tags, vec!["v1.0.0"]);

    // Deleting a tag on domain1 keeps the one of domain2
    let response = client.delete("/tags/v1.0.0")
        .header(Header::new("Authorization","Bearer test-token"))
        .header(Header::new("Host", "assets.domain1.com"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(!Path::new("./upload/assets.domain1.com/v1.0.0").exists());
    assert!(Path::new("./upload/assets.domain2.com/v1.0.0/test-file.txt").exists());

    cleanup_upload_dirs();
}

#[test]
#[serial]
fn per_host_unknown_host() {

    cleanup_upload_dirs();
    let client = Client::tracked(create_rocket(per_host_config())).unwrap();

    // Not in ALLOWED_HOSTS, or trying to escape the storage directory
    for host in ["assets.domain3.com", "..", "assets.domain1.com/.."] {
        assert_eq!(upload_on_host(&client, host, "v1.0.0/test-file.txt", "evil"), Status::NotFound, "host {}", host);

        let response = client.get("/tags").header(Header::new("Host", host)).dispatch();
        assert_eq!(response.status(), Status::NotFound, "host {}", host);
    }

    // No Host header at all
    let response = client.get("/tags").dispatch();
    assert_eq!(response.status(), Status::NotFound);

    // Nothing was written in the storage
    assert_eq!(std::fs::read_dir("./upload").unwrap().filter(|e| e.as_ref().unwrap().path().is_dir()).count(), 0);

    // Health stays available on any host
    let response = client.get("/health").header(Header::new("Host", "assets.domain3.com")).dispatch();
    assert_eq!(response.status(), Status::Ok);

    cleanup_upload_dirs();
}

#[test]
#[serial]
fn per_host_port_and_case() {

    cleanup_upload_dirs();
    let client = Client::tracked(create_rocket(per_host_config())).unwrap();

    assert_eq!(upload_on_host(&client, "Assets.Domain1.com:8000", "v1.0.0/test-file.txt", "domain1"), Status::Ok);
    assert!(Path::new("./upload/assets.domain1.com/v1.0.0/test-file.txt").exists());

    let response = client.get("/v1.0.0/test-file.txt").header(Header::new("Host", "assets.domain1.com")).dispatch();
    assert_eq!(response.status(), Status::Ok);

    cleanup_upload_dirs();
}


fn create_zip_slip_zip() -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);

    let options = SimpleFileOptions::default();

    // Escapes ./upload/v1.0.0 to ./upload
    zip.start_file("../zip-slip-evil.txt", options).unwrap();
    zip.write_all(b"evil").unwrap();

    // Escapes ./upload to the project root
    zip.start_file("../../zip-slip-evil.txt", options).unwrap();
    zip.write_all(b"evil").unwrap();

    zip.finish().unwrap();
    buffer.into_inner()
}

fn create_test_zip() -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);

    let options = SimpleFileOptions::default();

    // Add file1.txt
    zip.start_file("file1.txt", options).unwrap();
    zip.write_all(b"Hello world File 1!").unwrap();

    // Add file2.txt
    zip.start_file("file2.txt", options).unwrap();
    zip.write_all(b"Hello world File 2!").unwrap();

    zip.finish().unwrap();
    buffer.into_inner()
}


fn multipart_body(filename: &str, content: &str) -> (ContentType, String) {
    let boundary = "----TestBoundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n\
         {content}\r\n\
         --{boundary}--\r\n"
    );
    let content_type = ContentType::new("multipart", "form-data").with_params([("boundary", boundary)]);
    (content_type, body)
}

fn multipart_body_binary(filename: &str, content: &[u8], content_type: &str) -> (ContentType, Vec<u8>) {
    let boundary = "----TestBoundary";

    let mut body = Vec::new();
    body.extend_from_slice(format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
         Content-Type: {content_type}\r\n\r\n"
    ).as_bytes());
    body.extend_from_slice(content);  // Binary ZIP data
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let content_type = ContentType::new("multipart", "form-data").with_params([("boundary", boundary)]);
    (content_type, body)
}