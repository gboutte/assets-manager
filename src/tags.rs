use std::path::Path;
use rocket::{delete, get, State};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use crate::auth_guard::IsAuth;
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

#[delete("/<tag>")]
pub async fn delete_tag(
    _auth: IsAuth,
    tag: String,
    config: &State<Config>,
) -> Result<(), Status>{


    let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
    let full_path = base_path.join(&tag);


    if !full_path.starts_with(&base_path) {
        return Err(Status::BadRequest);  // Path traversal attempt (e.g., symlink escape)
    }


    std::fs::remove_dir(full_path).unwrap();

    Ok(())
}