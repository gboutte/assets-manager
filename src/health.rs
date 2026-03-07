use std::path::Path;
use rocket::{get, State};
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use crate::config::Config;

// Un struct pour la réponse
#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
}

// La route
#[get("/")]
pub fn health(config: &State<Config>) -> Json<HealthResponse> {
    let storage_ok = Path::new(&config.storage_path).exists();

    let status = if storage_ok { "ok" } else { "degraded" };
    let response =HealthResponse { status: status.to_string() };

    Json(response)
}