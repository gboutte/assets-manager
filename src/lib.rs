pub mod config;

pub mod storage;
pub mod assets;
pub mod health;
pub mod tags;
pub mod stats;
mod upload;
mod host;
mod auth_guard;

use rocket_cors::{AllowedHeaders, AllowedOrigins};
use rocket::fairing::AdHoc;
use rocket::http::Method;
use crate::stats::Stats;

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
        .attach(stats_fairing())
        .mount("/health", rocket::routes![health::health])
        .mount("/tags", rocket::routes![tags::tags_list, tags::tag_stats, tags::delete_tag])
        .mount("/upload", rocket::routes![upload::post_asset, upload::post_asset_zip, upload::reject_upload])
        .mount("/", rocket::routes![assets::get_asset])
        .manage(config)
}

// Opens the stats database at ignite (the server doesn't start if it fails),
// flushes pending hits periodically once launched, and a last time on shutdown
fn stats_fairing() -> AdHoc {
    AdHoc::try_on_ignite("Stats database", |rocket| async {
        let Some(config) = rocket.state::<config::Config>() else { return Err(rocket) };
        let stats = match Stats::open(&config.db_path) {
            Ok(stats) => stats,
            Err(e) => {
                log::error!("{}", e);
                return Err(rocket);
            }
        };
        stats.seed_existing_tags(config);

        Ok(rocket
            .manage(stats)
            .attach(AdHoc::on_liftoff("Stats flush", |rocket| Box::pin(async move {
                let Some(stats) = rocket.state::<Stats>().cloned() else { return };
                rocket::tokio::spawn(async move {
                    let mut interval = rocket::tokio::time::interval(stats::FLUSH_INTERVAL);
                    loop {
                        interval.tick().await;
                        stats.flush().await;
                    }
                });
            })))
            .attach(AdHoc::on_shutdown("Stats final flush", |rocket| Box::pin(async move {
                if let Some(stats) = rocket.state::<Stats>() {
                    stats.flush().await;
                }
            }))))
    })
}
