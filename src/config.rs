use std::env;

const API_TOKEN_PREFIX: &str = "API_TOKEN";
// One year: assets of a tag never change, browsers can keep them
const DEFAULT_CACHE_MAX_AGE: u64 = 31_536_000;

#[derive(Debug)]
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

        Ok(Config {
            api_tokens,
            storage_path: storage_path.unwrap(),
            storage_type: storage_type_value,
            cache_max_age,
            storage_per_host,
            allowed_hosts,
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
