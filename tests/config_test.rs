
#[cfg(test)]
mod tests {
    use std::env;
    use serial_test::serial;
    use assets_manager::config;

    #[test]
    #[serial]
    fn test_valid_config() {

        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
        }

        let config = match config::Config::from_env() {
            Ok(c) => c,
            Err(e) => panic!("Failed to load config: {}", e),
        };
        assert_eq!(config.api_token, "test-token");
        assert_eq!(config.storage_type, "filesystem");
        assert_eq!(config.storage_path, "./test-uploads")
    }

    #[test]
    #[serial]
    fn test_missing_api_token() {

        unsafe {
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
            env::remove_var("API_TOKEN");
        }

        let config = config::Config::from_env();
        assert!(config.is_err());
        assert!(config.unwrap_err().contains("API_TOKEN"));
    }

    #[test]
    #[serial]
    fn test_invalid_storage_type() {

        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "invalid");


            let config = config::Config::from_env();
            assert!(config.is_err());
            assert!(config.unwrap_err().contains("Invalid storage type"));
        }

    }
}