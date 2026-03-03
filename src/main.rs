mod config;
mod assets;

#[macro_use] extern crate rocket;

use rocket::http::Method;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use crate::assets::{get_asset, post_asset};



#[launch]
fn rocket() -> _ {

    if let Err(e) = dotenvy::dotenv() {
        println!("Could not load .env file: {}", e);
    }
    let config = match config::Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };
    println!("API Token: {}", config.api_token);
    println!("Storage Path: {}", config.storage_path);
    println!("Storage Type: {}", config.storage_type);



    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    }
        .to_cors().unwrap();

    rocket::build()
        .attach(cors)
        .mount("/", routes![get_asset])
        .mount("/", routes![post_asset])
        .manage(config)
}