use std::io;
use std::path::{Path, PathBuf};
use filesize::file_real_size;
use rocket::{get, post, uri};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::http::uri::Absolute;
use rocket::response::content::RawText;
use rocket::State;
use rocket::tokio::fs::File;
use crate::config::Config;

#[get("/<tag>/<file_name..>")]
pub async fn get_asset(tag: String, file_name:PathBuf, config: &State<Config>) -> Result<RawText<File>, Status> {

    let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
    let full_path = base_path.join(&tag).join(&file_name);

    println!("Full path: {}", full_path.display());
    println!("Base path: {}", base_path.display());

    // Resolve to absolute path (follows symlinks)
    let full_path = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(Status::NotFound),  // File doesn't exist
    };

    // Security: Ensure resolved path is within storage directory
    if !full_path.starts_with(&base_path) {
        return Err(Status::BadRequest);  // Path traversal attempt (e.g., symlink escape)
    }


    File::open(full_path).await.map(RawText).map_err(|_| Status::InternalServerError)
}


const HOST: Absolute<'static> = uri!("http://localhost:8000");


#[post("/<file_name_buf..>", data = "<file>")]
pub async fn post_asset(file_name_buf:PathBuf, mut file:TempFile<'_> ,config: &State<Config>)  -> io::Result<String> {

    let file_name = file_name_buf.clone().into_os_string().into_string().unwrap();
    let mimetype = file.content_type().unwrap();
    let filesize = file_real_size(file.path().unwrap()).unwrap();
    print!("File {}, type: {}, size: {}",file_name,mimetype,filesize);

    let full_path = Path::new(&config.storage_path).join(&file_name);

    let prefix = full_path.parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap();
    file.persist_to(full_path).await?;


    Ok(uri!(HOST ,get_asset("test",PathBuf::from(file_name))).to_string())
}