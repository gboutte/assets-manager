use std::env;

#[derive(Debug)]
pub struct Config {
    pub api_token: String,
    pub storage_path: String,
    pub storage_type: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {

        let mut missing = Vec::new();
        let api_token = env::var("API_TOKEN");
        let storage_path = env::var("STORAGE_PATH");
        let storage_type = env::var("STORAGE_TYPE");


        let valid_storage_types = ["filesystem"];

        if api_token.is_err(){
            missing.push("API_TOKEN");
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
            api_token: api_token.unwrap(),
            storage_path: storage_path.unwrap(),
            storage_type: storage_type_value,
        })

    }
}