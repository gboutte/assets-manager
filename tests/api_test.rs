
use rocket::http::{ContentType, Status};
use rocket::local::blocking::{Client};
use assets_manager::config::Config;
use assets_manager::create_rocket;
use rocket::serde::Deserialize;

#[derive(Deserialize)]
struct HealthResult {
    status: String,
}

#[test]
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
fn tags_list() {

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
    let tagsRespo = result.unwrap();
    assert_eq!(tagsRespo.tags.len(), 1);
    assert_eq!(tagsRespo.tags[0], "test-tag");


    //Clean up
    std::fs::remove_dir("./upload/test-tag").unwrap();


}