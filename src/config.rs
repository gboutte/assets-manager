use std::env;

const API_TOKEN_PREFIX: &str = "API_TOKEN";

#[derive(Debug)]
pub struct Config {
    pub api_tokens: Vec<String>,
    pub storage_path: String,
    pub storage_type: String,
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

        Ok(Config {
            api_tokens,
            storage_path: storage_path.unwrap(),
            storage_type: storage_type_value,
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
