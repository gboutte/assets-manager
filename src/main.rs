use assets_manager::config;
use assets_manager::create_rocket;



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
    
    create_rocket(config)
}


#[rocket::main]
async fn main() {
    let _ = rocket()
        .launch()
        .await;
}