use std::path::{Path, PathBuf};
use rocket::{get};
use rocket::fs::{NamedFile};
use rocket::http::Status;
use rocket::State;
use crate::config::Config;

#[get("/<tag>/<file_name..>")]
pub async fn get_asset(tag: String, file_name:PathBuf, config: &State<Config>) -> Result<NamedFile, Status> {

    let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
    let full_path = base_path.join(&tag).join(&file_name);

    println!("Full path: {}", full_path.display());
    println!("Base path: {}", base_path.display());

    // Resolve to absolute path (follows symlinks)
    let full_path = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(Status::NotFound),  // File doesn't exist
    };

    // Security: Ensure resolved path is within storage directory
    if !full_path.starts_with(&base_path) {
        return Err(Status::BadRequest);  // Path traversal attempt (e.g., symlink escape)
    }

    NamedFile::open(full_path).await.map_err(|_| Status::NotFound)
}
