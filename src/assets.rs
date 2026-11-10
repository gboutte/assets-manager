use std::path::{Path, PathBuf};
use rocket::{get, Responder};
use rocket::fs::{NamedFile};
use rocket::http::{Header, Status};
use rocket::State;
use crate::config::Config;
use crate::storage::validate_tag;

// A served file with its Cache-Control header
#[derive(Responder)]
pub struct CachedFile {
    file: NamedFile,
    cache_control: Header<'static>,
}

fn cache_control_header(max_age: u64) -> Header<'static> {
    let value = if max_age == 0 {
        "no-cache".to_string()
    } else {
        // Files of a tag never change, so the browser doesn't even need to revalidate them
        format!("public, max-age={}, immutable", max_age)
    };
    Header::new("Cache-Control", value)
}

#[get("/<tag>/<file_name..>")]
pub async fn get_asset(tag: String, file_name:PathBuf, config: &State<Config>) -> Result<CachedFile, Status> {

    validate_tag(&tag)?;

    let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
    let full_path = base_path.join(&tag).join(&file_name);

    // Resolve to absolute path (follows symlinks)
    let full_path = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(Status::NotFound),  // File doesn't exist
    };

    // Security: Ensure resolved path is within storage directory
    if !full_path.starts_with(&base_path) {
        log::warn!("Path traversal attempt blocked: {}/{}", tag, file_name.display());
        return Err(Status::BadRequest);  // Path traversal attempt (e.g., symlink escape)
    }

    let file = NamedFile::open(full_path).await.map_err(|_| Status::NotFound)?;

    Ok(CachedFile {
        file,
        cache_control: cache_control_header(config.cache_max_age),
    })
}
