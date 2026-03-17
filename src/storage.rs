// src/storage.rs
use std::path::{Component, Path, PathBuf};
use rocket::http::Status;

pub struct StoragePath {
    base: PathBuf,
}

impl StoragePath {
    pub fn new(storage_path: &str) -> Result<Self, Status> {
        let base = Path::new(storage_path)
            .canonicalize()
            .map_err(|_| Status::InternalServerError)?;
        Ok(Self { base })
    }

    pub fn resolve(&self, path:&str) -> Result<PathBuf, Status> {
        let path_joined = self.base.clone().join(path);
        let normalized_path = StoragePath::normalize_path(path_joined);

        if !normalized_path.starts_with(&self.base) {
            return Err(Status::BadRequest);
        }
        Ok(normalized_path)
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
        let ends_with_slash = path.as_ref()
            .to_str()
            .map_or(false, |s| s.ends_with('/'));
        let mut normalized = PathBuf::new();
        for component in path.as_ref().components() {
            match &component {
                Component::ParentDir => {
                    if !normalized.pop() {
                        normalized.push(component);
                    }
                }
                _ => {
                    normalized.push(component);
                }
            }
        }
        if ends_with_slash {
            normalized.push("");
        }
        normalized
    }
}