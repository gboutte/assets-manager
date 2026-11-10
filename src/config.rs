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

        Ok(Config {
            api_tokens,
            storage_path: storage_path.unwrap(),
            storage_type: storage_type_value,
            cache_max_age,
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
}
