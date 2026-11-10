use std::io::ErrorKind;
use rocket::{delete, get, State};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use crate::auth_guard::IsAuth;
use crate::stats::{Stats, TagStats};
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

// Option<IsAuth> instead of IsAuth: a refused IsAuth forwards the request, and the next
// matching route (GET /<tag>/<file..>) would answer 404 instead of 401.
// The token is checked first, so the response doesn't reveal whether the tag exists
#[get("/<tag>/stats")]
pub async fn tag_stats(
    auth: Option<IsAuth>,
    tag: String,
    storage: StoragePath,
    stats: &State<Stats>,
) -> Result<Json<TagStats>, Status> {

    if auth.is_none() {
        return Err(Status::Unauthorized);
    }
    validate_tag(&tag)?;

    if !storage.resolve(&tag)?.is_dir() {
        return Err(Status::NotFound);
    }

    // Stats disabled: the route behaves as if it didn't exist
    stats.tag_stats(storage.host(), &tag).await.map(Json).ok_or(Status::NotFound)
}

#[delete("/<tag>")]
pub async fn delete_tag(
    _auth: IsAuth,
    tag: String,
    storage: StoragePath,
    stats: &State<Stats>,
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

    stats.remove_tag(storage.host(), &tag).await;
    log::info!("Tag deleted: {}", tag);

    Ok(())
}