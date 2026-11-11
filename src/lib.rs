pub mod config;

pub mod storage;
pub mod assets;
pub mod health;
pub mod tags;
pub mod stats;
pub mod cleanup;
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
        .attach(cleanup_fairing())
        .mount("/health", rocket::routes![health::health])
        .mount("/tags", rocket::routes![tags::tags_list, tags::tag_stats, tags::delete_tag])
        .mount("/cleanup", rocket::routes![cleanup::cleanup_now])
        .mount("/upload", rocket::routes![upload::post_asset, upload::post_asset_zip, upload::reject_upload])
        .mount("/", rocket::routes![assets::get_asset])
        .manage(config)
}

// Opens the stats database at ignite (the server doesn't start if it fails),
// flushes pending hits periodically once launched, and a last time on shutdown
fn stats_fairing() -> AdHoc {
    AdHoc::try_on_ignite("Stats database", |rocket| async {
        let Some(config) = rocket.state::<config::Config>() else { return Err(rocket) };
        if !config.stats_needed() {
            log::info!("Stats disabled (set STATS_ENABLED=true or a CLEANUP_* rule to enable them)");
            return Ok(rocket.manage(Stats::disabled()));
        }
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

// Runs the cleanup rules every CLEANUP_INTERVAL_MINUTES once the server is launched.
// The first run happens after one interval, not at startup, so a restart never deletes anything right away
fn cleanup_fairing() -> AdHoc {
    AdHoc::on_liftoff("Cleanup task", |rocket| Box::pin(async move {
        let (Some(config), Some(stats)) = (rocket.state::<config::Config>(), rocket.state::<Stats>()) else { return };
        let rules = &config.cleanup;
        if !rules.has_rules() || rules.interval_minutes == 0 {
            log::info!("Auto-cleanup disabled");
            return;
        }
        log::info!(
            "Auto-cleanup every {} minute(s){}",
            rules.interval_minutes,
            if rules.dry_run { " (dry run: nothing is deleted)" } else { "" }
        );

        let config = config.clone();
        let stats = stats.clone();
        rocket::tokio::spawn(async move {
            let period = std::time::Duration::from_secs(config.cleanup.interval_minutes * 60);
            let start = rocket::tokio::time::Instant::now() + period;
            let mut interval = rocket::tokio::time::interval_at(start, period);
            loop {
                interval.tick().await;
                let deleted = cleanup::run_cleanup(&config, &stats, config.cleanup.dry_run).await;
                if !deleted.is_empty() {
                    log::info!("Cleanup: {} tag(s) {}", deleted.len(), if config.cleanup.dry_run { "would be deleted" } else { "deleted" });
                }
            }
        });
    }))
}
