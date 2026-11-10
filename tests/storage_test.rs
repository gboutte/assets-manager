#[cfg(test)]
mod tests {
    use std::path::Path;
    use rocket::http::Status;
    use serial_test::serial;
    use assets_manager::storage::{normalize_host, validate_tag, StoragePath};

    #[test]
    #[serial]
    fn test_storage_valid() {
        let storage_path_res = StoragePath::new("./upload");

        assert!(!storage_path_res.is_err());

        let storage_path = storage_path_res.unwrap();



        let resolved_path = storage_path.resolve("test-tag/test-file.txt");
        assert!(resolved_path.is_ok());

        let resolved_path = resolved_path.unwrap();
        let resolved_path_str = resolved_path.to_str().unwrap();


        let test_path = Path::new("./upload").canonicalize().unwrap();
        let joined_path = test_path.join("test-tag/test-file.txt");
        let test_path_str = joined_path.to_str().unwrap();

         assert!(resolved_path_str == test_path_str, "Resolved path: {}, Test path: {}", resolved_path_str, test_path_str);

    }

    #[test]
    #[serial]
    fn test_storage_bad_request() {
        let storage_path_res = StoragePath::new("./upload");

        assert!(!storage_path_res.is_err());

        let storage_path = storage_path_res.unwrap();



        let resolved_path = storage_path.resolve("../");
        assert!(resolved_path.is_err());
        assert!(resolved_path.unwrap_err() == Status::BadRequest);
    }
    #[test]
    #[serial]
    fn test_storage_normalize_path() {
       let path1 = "/test/home/dir1/../dir2";

        let normalized_path1 = StoragePath::normalize_path(path1);
        assert_eq!(normalized_path1.to_str().unwrap(), "/test/home/dir2");
    }

    #[test]
    fn test_validate_tag_valid() {
        for tag in ["v1.0.0", "release-2026_11", "a", &"a".repeat(64)] {
            assert_eq!(validate_tag(tag), Ok(tag), "{} should be valid", tag);
        }
    }

    #[test]
    fn test_validate_tag_invalid() {
        let too_long = "a".repeat(65);
        for tag in ["", ".", "..", ".hidden", "a/b", "a\\b", "with space", "v1@0", "é", &too_long] {
            assert_eq!(validate_tag(tag), Err(Status::BadRequest), "{} should be invalid", tag);
        }
    }

    #[test]
    fn test_normalize_host() {
        assert_eq!(normalize_host("assets.domain1.com"), "assets.domain1.com");
        assert_eq!(normalize_host("Assets.Domain1.COM"), "assets.domain1.com");
        assert_eq!(normalize_host("assets.domain1.com:8000"), "assets.domain1.com");
        assert_eq!(normalize_host("localhost:80"), "localhost");
    }
}
