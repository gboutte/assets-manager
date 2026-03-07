use std::io;
use std::path::{Path, PathBuf};
use filesize::file_real_size;
use rocket::{get, post, uri};
use rocket::fs::TempFile;
use rocket::http::uri::Absolute;
use rocket::response::content::RawText;
use rocket::State;
use rocket::tokio::fs::File;
use crate::config::Config;

#[get("/<file_name..>")]
pub async fn get_asset(file_name:PathBuf, config: &State<Config>) -> Option<RawText<File>> {
    let full_path = Path::new(&config.storage_path).join(file_name);

    println!("fullPath: {:?}", full_path);

    File::open(full_path).await.map(RawText).ok()
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


    Ok(uri!(HOST ,get_asset(PathBuf::from(file_name))).to_string())
}