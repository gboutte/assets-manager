use std::path::{Path, PathBuf};
use filesize::file_real_size;
use rocket::{post, uri, FromForm};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use rocket::State;
use zip::ZipArchive;
use crate::assets;
use crate::auth_guard::IsAuth;
use crate::config::Config;
use crate::host::RequestInfo;

#[derive(FromForm)]
pub struct Upload<'f> {
    file: TempFile<'f>
}

#[post("/<tag>/<file_name_buf..>", format = "multipart/form-data", data = "<form>", rank = 2)]
pub async fn post_asset(
    _auth: IsAuth,
    tag: String,
    file_name_buf:PathBuf,
    mut form: Form<Upload<'_>> ,
    config: &State<Config>,
    request_info: RequestInfo,
)  -> Result<String, Status> {

    let file_name = file_name_buf.clone().into_os_string().into_string().map_err(|_| Status::InternalServerError)?;
    let mimetype = form.file.content_type().cloned();
    let uploaded_filepath = form.file.path().ok_or_else(|| Status::InternalServerError)?;

    let filesize = file_real_size(uploaded_filepath).map_err(|_| Status::InternalServerError)?;

    let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
    let full_path = base_path.join(&tag).join(&file_name);


    if !full_path.starts_with(&base_path) {
        return Err(Status::BadRequest);  // Path traversal attempt (e.g., symlink escape)
    }


    let prefix = full_path.parent().ok_or_else(|| Status::InternalServerError)?;
    std::fs::create_dir_all(prefix).map_err(|_| Status::InternalServerError)?;
    let _ = form.file.persist_to(full_path).await;

    let path = uri!(assets::get_asset(&tag, PathBuf::from(&file_name)));
    let full_url = format!("{}://{}{}", request_info.protocol, request_info.host, path);


    print!(
        "File {}, type: {}, size: {}",
        full_url,
        mimetype.unwrap_or(ContentType::Binary),
        filesize);

    Ok(full_url.to_string())
}



#[post("/<tag>", format = "multipart/form-data", data = "<form>", rank = 1)]
pub async fn post_asset_zip(
    _auth: IsAuth,
    tag: String,
    form: Form<Upload<'_>> ,
    config: &State<Config>,
)  -> Result< String,Status> {


    let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
    let full_path = base_path.join(&tag);


    if !full_path.starts_with(&base_path) {
        return Err(Status::BadRequest);  // Path traversal attempt (e.g., symlink escape)
    }


    let prefix = full_path.parent().ok_or_else(|| Status::InternalServerError)?;
    std::fs::create_dir_all(prefix).map_err(|_| Status::InternalServerError)?;


    //Check that the file is a zip
    let uploaded_filepath = form.file.path().ok_or_else(|| Status::InternalServerError)?;

    let file = std::fs::File::open(uploaded_filepath).map_err(|_| Status::InternalServerError)?;
    let mut zip = ZipArchive::new(file).map_err(|_| Status::InternalServerError)?;

    zip.extract(&full_path).map_err(|_| Status::InternalServerError)?;


    Ok(format!("Extracted {} files to {}", zip.len(), tag))
}