// src/storage.rs
use std::path::{Component, Path, PathBuf};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use crate::config::Config;

pub struct StoragePath {
    base: PathBuf,
    // Host of the request when STORAGE_PER_HOST is enabled, "" otherwise
    host: String,
}

impl StoragePath {
    pub fn new(storage_path: &str) -> Result<Self, Status> {
        let base = Path::new(storage_path)
            .canonicalize()
            .map_err(|_| Status::InternalServerError)?;
        Ok(Self { base, host: String::new() })
    }

    pub fn host(&self) -> &str {
        &self.host
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

// Request guard giving the storage directory of the request:
// <storage_path> by default, <storage_path>/<host> when STORAGE_PER_HOST is enabled.
// A host that is not in ALLOWED_HOSTS gets a 404, as if the site didn't exist
#[rocket::async_trait]
impl<'r> FromRequest<'r> for StoragePath {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let config = match request.guard::<&rocket::State<Config>>().await {
            Outcome::Success(c) => c,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

        if !config.storage_per_host {
            return match StoragePath::new(&config.storage_path) {
                Ok(storage) => Outcome::Success(storage),
                Err(status) => Outcome::Error((status, ())),
            };
        }

        let host = request.headers().get_one("Host").map(normalize_host).unwrap_or_default();
        // The host is compared to the configured list, so a forged Host header can't
        // be used to reach another directory
        if !config.allowed_hosts.contains(&host) {
            log::warn!("Host not allowed: {:?}", host);
            return Outcome::Error((Status::NotFound, ()));
        }

        let host_path = Path::new(&config.storage_path).join(&host);
        if let Err(e) = std::fs::create_dir_all(&host_path) {
            log::error!("Failed to create storage directory for host {}: {}", host, e);
            return Outcome::Error((Status::InternalServerError, ()));
        }

        match host_path.to_str().map(StoragePath::new) {
            Some(Ok(storage)) => Outcome::Success(StoragePath { host, ..storage }),
            _ => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

// "Assets.Domain1.com:8000" -> "assets.domain1.com"
pub fn normalize_host(host: &str) -> String {
    let without_port = match host.rsplit_once(':') {
        Some((name, port)) if port.chars().all(|c| c.is_ascii_digit()) => name,
        _ => host,
    };
    without_port.trim().to_lowercase()
}

const TAG_MAX_LENGTH: usize = 64;

// A tag is a single directory name: ASCII letters, digits, '-', '_' and '.', not starting with '.'
// (rejects "", ".", "..", hidden directories and any path separator)
pub fn validate_tag(tag: &str) -> Result<&str, Status> {
    let valid = !tag.is_empty()
        && tag.len() <= TAG_MAX_LENGTH
        && !tag.starts_with('.')
        && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');

    if !valid {
        log::warn!("Invalid tag rejected: {:?}", tag);
        return Err(Status::BadRequest);
    }
    Ok(tag)
}
