
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
        unsafe {
            env::remove_var("CACHE_MAX_AGE");
            env::remove_var("STORAGE_PER_HOST");
            env::remove_var("ALLOWED_HOSTS");
            env::remove_var("DB_PATH");
            env::remove_var("STATS_ENABLED");
            for name in ["CLEANUP_MAX_AGE_DAYS", "CLEANUP_MAX_IDLE_DAYS", "CLEANUP_KEEP_LAST",
                         "CLEANUP_KEEP_TAGS", "CLEANUP_INTERVAL_MINUTES", "CLEANUP_DRY_RUN"] {
                env::remove_var(name);
            }
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
        assert_eq!(config.storage_path, "./test-uploads");
        assert_eq!(config.cache_max_age, 31536000);
        assert!(!config.storage_per_host);
        assert_eq!(config.db_path, "./test-uploads/.assets-manager.db");
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

    #[test]
    #[serial]
    fn test_cache_max_age() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
            env::set_var("CACHE_MAX_AGE", "3600");
        }

        let config = config::Config::from_env().expect("valid config");
        assert_eq!(config.cache_max_age, 3600);

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_invalid_cache_max_age() {

        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
            env::set_var("CACHE_MAX_AGE", "one-hour");
        }

        let config = config::Config::from_env();
        assert!(config.is_err());
        assert!(config.unwrap_err().contains("Invalid CACHE_MAX_AGE"));

        clear_api_tokens();
    }

    fn set_base_env() {
        clear_api_tokens();
        unsafe {
            env::set_var("API_TOKEN", "test-token");
            env::set_var("STORAGE_PATH", "./test-uploads");
            env::set_var("STORAGE_TYPE", "filesystem");
        }
    }

    #[test]
    #[serial]
    fn test_storage_per_host() {

        set_base_env();
        unsafe {
            env::set_var("STORAGE_PER_HOST", "true");
            env::set_var("ALLOWED_HOSTS", " Assets.Domain1.com , assets.domain2.com,");
        }

        let config = config::Config::from_env().expect("valid config");
        assert!(config.storage_per_host);
        assert_eq!(config.allowed_hosts, vec!["assets.domain1.com", "assets.domain2.com"]);

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_storage_per_host_without_allowed_hosts() {

        set_base_env();
        unsafe { env::set_var("STORAGE_PER_HOST", "true"); }

        let config = config::Config::from_env();
        assert!(config.unwrap_err().contains("ALLOWED_HOSTS is required"));

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_invalid_storage_per_host() {

        set_base_env();
        unsafe { env::set_var("STORAGE_PER_HOST", "yes please"); }

        let config = config::Config::from_env();
        assert!(config.unwrap_err().contains("Invalid STORAGE_PER_HOST"));

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_invalid_allowed_host() {

        for host in ["..", "domain1.com/..", ".hidden.com", "domain 1.com"] {
            set_base_env();
            unsafe {
                env::set_var("STORAGE_PER_HOST", "true");
                env::set_var("ALLOWED_HOSTS", host);
            }

            let config = config::Config::from_env();
            assert!(config.unwrap_err().contains("Invalid host in ALLOWED_HOSTS"), "host {}", host);
        }

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_db_path() {

        set_base_env();
        unsafe { env::set_var("DB_PATH", "/var/lib/assets/stats.db"); }

        let config = config::Config::from_env().expect("valid config");
        assert_eq!(config.db_path, "/var/lib/assets/stats.db");

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_cleanup_default() {

        set_base_env();
        let cleanup = config::Config::from_env().expect("valid config").cleanup;
        assert!(!cleanup.has_rules());
        assert_eq!(cleanup.keep_last, 0);
        assert_eq!(cleanup.interval_minutes, 60);
        assert!(!cleanup.dry_run);

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_cleanup_rules() {

        set_base_env();
        unsafe {
            env::set_var("CLEANUP_MAX_AGE_DAYS", "90");
            env::set_var("CLEANUP_MAX_IDLE_DAYS", "30");
            env::set_var("CLEANUP_KEEP_LAST", "5");
            env::set_var("CLEANUP_KEEP_TAGS", " prod, v1.* ,");
            env::set_var("CLEANUP_INTERVAL_MINUTES", "15");
            env::set_var("CLEANUP_DRY_RUN", "true");
        }

        let cleanup = config::Config::from_env().expect("valid config").cleanup;
        assert!(cleanup.has_rules());
        assert_eq!(cleanup.max_age_days, Some(90));
        assert_eq!(cleanup.max_idle_days, Some(30));
        assert_eq!(cleanup.keep_last, 5);
        assert_eq!(cleanup.keep_tags, vec!["prod", "v1.*"]);
        assert_eq!(cleanup.interval_minutes, 15);
        assert!(cleanup.dry_run);

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_invalid_cleanup_numbers() {

        for (name, value) in [("CLEANUP_MAX_AGE_DAYS", "-1"), ("CLEANUP_MAX_IDLE_DAYS", "ten"),
                              ("CLEANUP_KEEP_LAST", "1.5"), ("CLEANUP_INTERVAL_MINUTES", "x")] {
            set_base_env();
            unsafe { env::set_var(name, value); }

            let config = config::Config::from_env();
            assert!(config.unwrap_err().contains(&format!("Invalid {}", name)), "{}={}", name, value);
            unsafe { env::remove_var(name); }
        }

        clear_api_tokens();
    }

    #[test]
    #[serial]
    fn test_stats_needed() {

        // Neither stats nor cleanup: no database
        set_base_env();
        let config = config::Config::from_env().expect("valid config");
        assert!(!config.stats_enabled);
        assert!(!config.stats_needed());

        // Explicitly enabled
        unsafe { env::set_var("STATS_ENABLED", "true"); }
        assert!(config::Config::from_env().unwrap().stats_needed());

        // Enabled by a cleanup rule, even with STATS_ENABLED=false
        unsafe {
            env::set_var("STATS_ENABLED", "false");
            env::set_var("CLEANUP_MAX_IDLE_DAYS", "30");
        }
        let config = config::Config::from_env().unwrap();
        assert!(!config.stats_enabled);
        assert!(config.stats_needed());

        // keep_* alone are not rules
        unsafe {
            env::remove_var("CLEANUP_MAX_IDLE_DAYS");
            env::set_var("CLEANUP_KEEP_LAST", "3");
        }
        assert!(!config::Config::from_env().unwrap().stats_needed());

        unsafe { env::set_var("STATS_ENABLED", "maybe"); }
        assert!(config::Config::from_env().unwrap_err().contains("Invalid STATS_ENABLED"));

        clear_api_tokens();
    }
}
