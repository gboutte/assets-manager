use std::env;
use rocket::figment::Figment;
use assets_manager::config;
use assets_manager::create_rocket;


// HOST and PORT override Rocket's own ROCKET_ADDRESS / ROCKET_PORT (default 127.0.0.1:8000)
fn server_figment() -> Result<Figment, String> {
    let mut figment = rocket::Config::figment();

    if let Ok(host) = env::var("HOST") {
        figment = figment.merge(("address", host));
    }
    if let Ok(port) = env::var("PORT") {
        let port = port.parse::<u16>().map_err(|_| format!("Invalid PORT: {}", port))?;
        figment = figment.merge(("port", port));
    }

    Ok(figment)
}

fn rocket() -> rocket::Rocket<rocket::Build> {

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
    let figment = match server_figment() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };
    println!("{} API token(s) loaded", config.api_tokens.len());

    create_rocket(config).configure(figment)
}


#[rocket::main]
async fn main() {
    // Exit with an error code if the server can't start (e.g. port already in use)
    if let Err(e) = rocket().launch().await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
