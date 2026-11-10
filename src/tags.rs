use std::io::ErrorKind;
use rocket::{delete, get};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use crate::auth_guard::IsAuth;
use crate::storage::{validate_tag, StoragePath};

// Un struct pour la réponse
#[derive(Serialize)]
pub struct TagsResponse {
    tags: Vec<String>,
}

// La route
#[get("/")]
pub fn tags_list(storage: StoragePath) -> Result<Json<TagsResponse>, Status> {
    // We get only the directory from storage path
    let tags = std::fs::read_dir(storage.base())
        .map_err(|e| {
            log::error!("Failed to read storage directory: {}", e);
            Status::InternalServerError
        })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                entry.file_name().into_string().ok()
            } else {
                None
            }
        })
         .collect();

    Ok(Json(TagsResponse { tags }))
}

#[delete("/<tag>")]
pub async fn delete_tag(
    _auth: IsAuth,
    tag: String,
    storage: StoragePath,
) -> Result<(), Status>{

    validate_tag(&tag)?;

    let full_path = storage.resolve(&tag)?;


    std::fs::remove_dir_all(full_path).map_err(|e| match e.kind() {
        ErrorKind::NotFound => Status::NotFound,
        _ => {
            log::error!("Failed to delete tag {}: {}", tag, e);
            Status::InternalServerError
        }
    })?;

    log::info!("Tag deleted: {}", tag);

    Ok(())
}