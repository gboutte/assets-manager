use std::path::Path;
use rocket::{get, State};
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use crate::config::Config;

// Un struct pour la réponse
#[derive(Serialize)]
pub struct TagsResponse {
    tags: Vec<String>,
}

// La route
#[get("/")]
pub fn tags_list(config: &State<Config>) -> Json<TagsResponse> {
    let storage_ok = Path::new(&config.storage_path).exists();
    if !storage_ok {
        return Json(TagsResponse { tags: Vec::new() });
    }

    // We get only the directory from storage path
    let tags = std::fs::read_dir(&config.storage_path)
        .expect("Failed to read storage directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                entry.file_name().into_string().ok()
            } else {
                None
            }
        })
         .collect();

    Json(TagsResponse { tags })
}