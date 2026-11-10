use std::env;

const API_TOKEN_PREFIX: &str = "API_TOKEN";
// One year: assets of a tag never change, browsers can keep them
const DEFAULT_CACHE_MAX_AGE: u64 = 31_536_000;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_tokens: Vec<String>,
    pub storage_path: String,
    pub storage_type: String,
    // Cache-Control max-age in seconds for served assets, 0 disables caching
    pub cache_max_age: u64,
    // When true, assets are stored per domain in <storage_path>/<host>/<tag>/...
    pub storage_per_host: bool,
    // Domains served when storage_per_host is enabled (lowercase, without port)
    pub allowed_hosts: Vec<String>,
    // SQLite database keeping upload dates and hits of each tag
    pub db_path: String,
    // STATS_ENABLED; the stats database is also opened when a cleanup rule needs it
    pub stats_enabled: bool,
    pub cleanup: CleanupConfig,
}

impl Config {
    pub fn stats_needed(&self) -> bool {
        self.stats_enabled || self.cleanup.has_rules()
    }
}

// Auto-cleanup rules, see cleanup.rs. A tag is deleted when it is too old OR idle for too long,
// unless it is protected (one of the keep_last most recent, or matching keep_tags)
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    pub max_age_days: Option<u64>,
    pub max_idle_days: Option<u64>,
    pub keep_last: usize,
    // Tag names or patterns with '*', e.g. "prod", "v1.*"
    pub keep_tags: Vec<String>,
    // 0 disables the background task
    pub interval_minutes: u64,
    // Only logs what would be deleted
    pub dry_run: bool,
}

impl Default for CleanupConfig {
    // No rule: nothing is ever deleted
    fn default() -> Self {
        CleanupConfig {
            max_age_days: None,
            max_idle_days: None,
            keep_last: 0,
            keep_tags: Vec::new(),
            interval_minutes: 60,
            dry_run: false,
        }
    }
}

impl CleanupConfig {
    pub fn has_rules(&self) -> bool {
        self.max_age_days.is_some() || self.max_idle_days.is_some()
    }

    fn from_env() -> Result<Self, String> {
        let defaults = CleanupConfig::default();
        let dry_run = match env::var("CLEANUP_DRY_RUN") {
            Ok(value) => Config::parse_bool(&value)
                .ok_or_else(|| format!("Invalid CLEANUP_DRY_RUN: {}, expected true or false", value))?,
            Err(_) => defaults.dry_run,
        };

        Ok(CleanupConfig {
            max_age_days: optional_number("CLEANUP_MAX_AGE_DAYS")?,
            max_idle_days: optional_number("CLEANUP_MAX_IDLE_DAYS")?,
            keep_last: optional_number("CLEANUP_KEEP_LAST")?.unwrap_or(defaults.keep_last),
            keep_tags: env::var("CLEANUP_KEEP_TAGS").unwrap_or_default()
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect(),
            interval_minutes: optional_number("CLEANUP_INTERVAL_MINUTES")?.unwrap_or(defaults.interval_minutes),
            dry_run,
        })
    }
}

// An unset or empty variable is None, anything else must be a number.
// Generic over the number type: works for u64, usize... (anything that parses from a string)
fn optional_number<T: std::str::FromStr>(name: &str) -> Result<Option<T>, String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => value.trim().parse::<T>()
            .map(Some)
            .map_err(|_| format!("Invalid {}: {}, expected a positive number", name, value)),
        _ => Ok(None),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, String> {

        let mut missing = Vec::new();
        let api_tokens = Config::api_tokens_from_env();
        let storage_path = env::var("STORAGE_PATH");
        let storage_type = env::var("STORAGE_TYPE");


        let valid_storage_types = ["filesystem"];

        if api_tokens.is_empty(){
            missing.push("API_TOKEN (or any API_TOKEN* variable)");
        }
        if storage_path.is_err(){
            missing.push("STORAGE_PATH");
        }
        if storage_type.is_err(){
            missing.push("STORAGE_TYPE");
        }


        if !missing.is_empty(){
            return Err(format!("Missing environment variables: {}", missing.join(", ")));
        }
        let storage_type_value = storage_type.unwrap_or("filesystem".to_string());
        if !valid_storage_types.contains(&storage_type_value.as_str()){
            return Err(format!("Invalid storage type: {}, allowed values {}", storage_type_value, valid_storage_types.join(", ")));
        }

        let cache_max_age = match env::var("CACHE_MAX_AGE") {
            Ok(value) => value.parse::<u64>()
                .map_err(|_| format!("Invalid CACHE_MAX_AGE: {}, expected a number of seconds", value))?,
            Err(_) => DEFAULT_CACHE_MAX_AGE,
        };

        let storage_per_host = match env::var("STORAGE_PER_HOST") {
            Ok(value) => Config::parse_bool(&value)
                .ok_or_else(|| format!("Invalid STORAGE_PER_HOST: {}, expected true or false", value))?,
            Err(_) => false,
        };

        let allowed_hosts = Config::allowed_hosts_from_env()?;
        if storage_per_host && allowed_hosts.is_empty() {
            return Err("ALLOWED_HOSTS is required when STORAGE_PER_HOST is enabled".to_string());
        }

        let stats_enabled = match env::var("STATS_ENABLED") {
            Ok(value) => Config::parse_bool(&value)
                .ok_or_else(|| format!("Invalid STATS_ENABLED: {}, expected true or false", value))?,
            Err(_) => false,
        };

        let storage_path = storage_path.unwrap();
        // Starts with a dot so it is never listed or served as a tag
        let db_path = env::var("DB_PATH").unwrap_or_else(|_| format!("{}/.assets-manager.db", storage_path));

        Ok(Config {
            api_tokens,
            storage_path,
            storage_type: storage_type_value,
            cache_max_age,
            storage_per_host,
            allowed_hosts,
            db_path,
            stats_enabled,
            cleanup: CleanupConfig::from_env()?,
        })

    }

    // Every variable whose name starts with API_TOKEN is a valid token (API_TOKEN, API_TOKEN_CI, ...)
    // Empty values are ignored, otherwise "Bearer " alone would be accepted
    fn api_tokens_from_env() -> Vec<String> {
        env::vars_os()
            .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
            .filter(|(name, value)| name.starts_with(API_TOKEN_PREFIX) && !value.is_empty())
            .map(|(_, value)| value)
            .collect()
    }

    fn parse_bool(value: &str) -> Option<bool> {
        match value.to_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" | "" => Some(false),
            _ => None,
        }
    }

    // ALLOWED_HOSTS=assets.domain1.com,assets.domain2.com
    // Each host becomes a directory name, so only plain domain names are accepted
    fn allowed_hosts_from_env() -> Result<Vec<String>, String> {
        let value = env::var("ALLOWED_HOSTS").unwrap_or_default();
        let mut hosts = Vec::new();

        for host in value.split(',').map(|h| h.trim().to_lowercase()).filter(|h| !h.is_empty()) {
            let valid = !host.starts_with('.')
                && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
            if !valid {
                return Err(format!("Invalid host in ALLOWED_HOSTS: {}", host));
            }
            hosts.push(host);
        }
        Ok(hosts)
    }
}
