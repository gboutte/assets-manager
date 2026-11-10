
#[cfg(test)]
mod tests {
    use std::env;
    use serial_test::serial;
    use assets_manager::config;

    // Removes every API_TOKEN* variable so tests don't see tokens set by other tests
    fn clear_api_tokens() {
        let names: Vec<String> = env::vars()
            .map(|(name, _)| name)
            .filter(|name| name.starts_with("API_TOKEN"))
            .collect();
        for name in names {
            unsafe { env::remove_var(name); }
        }
    }

    #[test]
    #[serial]
    fn test_valid_config() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
        }

        let config = match config::Config::from_env() {
            Ok(c) => c,
            Err(e) => panic!("Failed to load config: {}", e),
        };
        assert_eq!(config.api_tokens, vec!["test-token"]);
        assert_eq!(config.storage_type, "filesystem");
        assert_eq!(config.storage_path, "./test-uploads")
    }

    #[test]
    #[serial]
    fn test_missing_api_token() {

        clear_api_tokens();
        unsafe {
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
        }

        let config = config::Config::from_env();
        assert!(config.is_err());
        assert!(config.unwrap_err().contains("API_TOKEN"));
    }

    #[test]
    #[serial]
    fn test_invalid_storage_type() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "invalid");


            let config = config::Config::from_env();
            assert!(config.is_err());
            assert!(config.unwrap_err().contains("Invalid storage type"));
        }

    }
    #[test]
    #[serial]
    fn test_no_storage_path() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_TYPE", "filesystem");
            env::remove_var("STORAGE_PATH");


            let config = config::Config::from_env();
            assert!(config.is_err());
            assert!(config.unwrap_err().contains("Missing environment variables: STORAGE_PATH"));
        }

    }
    #[test]
    #[serial]
    fn test_no_storage_type() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::remove_var("STORAGE_TYPE");


            let config = config::Config::from_env();
            assert!(config.is_err());
            assert!(config.unwrap_err().contains("Missing environment variables: STORAGE_TYPE"));
        }

    }

    #[test]
    #[serial]
    fn test_multiple_api_tokens() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "token-main");
            env::set_var("API_TOKEN_CI", "token-ci");
            env::set_var("API_TOKEN_A8F3K2", "token-random");
            env::set_var("API_TOKEN_EMPTY", "");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
        }

        let config = config::Config::from_env().expect("valid config");
        let mut tokens = config.api_tokens.clone();
        tokens.sort();
        assert_eq!(tokens, vec!["token-ci", "token-main", "token-random"]);

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_only_prefixed_api_token() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN_DEPLOY", "token-deploy");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
        }

        let config = config::Config::from_env().expect("valid config");
        assert_eq!(config.api_tokens, vec!["token-deploy"]);

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_only_empty_api_token() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
        }

        let config = config::Config::from_env();
        assert!(config.is_err());
        assert!(config.unwrap_err().contains("API_TOKEN"));

        clear_api_tokens();
    }
}
