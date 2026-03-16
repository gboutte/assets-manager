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
        api_token: "test-token".to_string(),
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
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
        api_token: "test-token".to_string(),
        storage_path: "./test-uploads".to_string(),
        storage_type: "filesystem".to_string(),
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
        api_token: "test-token".to_string(),
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
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
fn file_get() {


    cleanup_upload_dirs();


    let config: Config = Config {
        api_token: "test-token".to_string(),
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
    };

    let client = Client::tracked(create_rocket(config)).expect("valid rocket instance");
    let response = client.get("/v1.0.0/test-file.txt").dispatch();
    assert_eq!(response.status(), Status::NotFound,"/v1.0.0/test-file.txt should be not found");



    std::fs::create_dir("./upload/v1.0.0").unwrap();
    std::fs::write("./upload/v1.0.0/test-file.txt", "test").unwrap();

    let response = client.get("/v1.0.0/test-file.txt").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("test".to_string()));



    //Clean up
    cleanup_upload_dirs();

}
#[test]
#[serial]
fn file_get_path_traversal() {


    cleanup_upload_dirs();


    let config: Config = Config {
        api_token: "test-token".to_string(),
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
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
        api_token: "test-token".to_string(),
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
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
        api_token: "test-token".to_string(),
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
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
fn test_upload_zip() {


    //Clean up
    cleanup_upload_dirs();

    let config: Config = Config {
        api_token: "test-token".to_string(),
        storage_path: "./upload".to_string(),
        storage_type: "filesystem".to_string(),
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