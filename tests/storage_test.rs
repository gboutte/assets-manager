#[cfg(test)]
mod tests {
    use std::path::Path;
    use rocket::http::Status;
    use serial_test::serial;
    use assets_manager::storage::StoragePath;

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


}