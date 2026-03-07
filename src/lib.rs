pub mod config;
pub mod assets;

use rocket_cors::{AllowedHeaders, AllowedOrigins};
use rocket::http::Method;

pub fn create_rocket(config: config::Config) -> rocket::Rocket<rocket::Build> {
    let allowed_origins = AllowedOrigins::all();
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    }.to_cors().unwrap();

    rocket::build()
        .attach(cors)
        .mount("/", rocket::routes![assets::get_asset, assets::post_asset])
        .manage(config)
}