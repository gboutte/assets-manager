use std::path::{Path, PathBuf};
use filesize::file_real_size;
use rocket::{post, uri, FromForm};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::http::uri::Absolute;
use rocket::State;
use crate::assets;
use crate::config::Config;



const HOST: Absolute<'static> = uri!("http://localhost:8000");
#[derive(FromForm)]
pub struct Upload<'f> {
    file: TempFile<'f>
}

#[post("/<tag>/<file_name_buf..>", format = "multipart/form-data", data = "<form>")]
pub async fn post_asset(tag: String,file_name_buf:PathBuf, mut form: Form<Upload<'_>> ,config: &State<Config>)  -> Result<String, Status> {

    let file_name = file_name_buf.clone().into_os_string().into_string().unwrap();
    let mimetype = form.file.content_type().cloned();
    let filesize = file_real_size(form.file.path().unwrap()).unwrap();

    let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
    let full_path = base_path.join(&tag).join(&file_name);


    if !full_path.starts_with(&base_path) {
        return Err(Status::BadRequest);  // Path traversal attempt (e.g., symlink escape)
    }


    let prefix = full_path.parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap();
    let _ = form.file.persist_to(full_path).await;

    let uri = uri!(HOST ,assets::get_asset(tag,PathBuf::from(file_name)));


    print!("File {}, type: {}, size: {}",uri,mimetype.unwrap(),filesize);

    Ok(uri.to_string())
}