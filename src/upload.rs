use std::path::{Path, PathBuf};
use rocket::{post, uri, FromForm, State};
use rocket::data::{Data, Limits, ToByteUnit};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use zip::ZipArchive;
use zip::result::ZipError;
use crate::assets;
use crate::auth_guard::IsAuth;
use crate::host::RequestInfo;
use crate::stats::Stats;
use crate::storage::{validate_tag, StoragePath};

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
    storage_path: StoragePath,
    request_info: RequestInfo,
    stats: &State<Stats>,
)  -> Result<String, Status> {

    validate_tag(&tag)?;

    let file_name = file_name_buf.clone().into_os_string().into_string().map_err(|_| Status::InternalServerError)?;
    let mimetype = form.file.content_type().cloned();
    let filesize = form.file.len();

    let file_path = Path::new(&tag).join(&file_name);
    let file_path_str =file_path.to_str().ok_or_else(|| Status::BadRequest)?;

    let full_path = storage_path.resolve(file_path_str).map_err(|_| Status::BadRequest)?;


    let prefix = full_path.parent().ok_or_else(|| Status::InternalServerError)?;
    std::fs::create_dir_all(prefix).map_err(|_| Status::InternalServerError)?;
    // Not persist_to: it renames the temp file, which fails when the storage is on another
    // filesystem than the temp dir (e.g. a Docker volume). move_copy_to copies then deletes it
    form.file.move_copy_to(&full_path).await.map_err(|e| {
        log::error!("Failed to write file {}: {}", full_path.display(), e);
        Status::InternalServerError
    })?;

    stats.record_upload(storage_path.host(), &tag).await;

    let path = uri!(assets::get_asset(&tag, PathBuf::from(&file_name)));
    let full_url = format!("{}://{}{}", request_info.protocol, request_info.host, path);


    log::info!(
        "File uploaded: {}, type: {}, size: {} bytes",
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
    storage_path: StoragePath,
    stats: &State<Stats>,
)  -> Result< String,Status> {
    validate_tag(&tag)?;
    let full_path = storage_path.resolve(&tag).map_err(|_| Status::BadRequest)?;


    let prefix = full_path.parent().ok_or_else(|| Status::InternalServerError)?;
    std::fs::create_dir_all(prefix).map_err(|_| Status::InternalServerError)?;


    //Check that the file is a zip
    let uploaded_filepath = form.file.path().ok_or_else(|| Status::InternalServerError)?;

    let file = std::fs::File::open(uploaded_filepath).map_err(|_| Status::InternalServerError)?;
    let mut zip = ZipArchive::new(file).map_err(zip_error_status)?;

    zip.extract(&full_path).map_err(zip_error_status)?;


    stats.record_upload(storage_path.host(), &tag).await;
    log::info!("Zip extracted: {} files to tag {}", zip.len(), tag);

    Ok(format!("Extracted {} files to {}", zip.len(), tag))
}

// A malformed or malicious archive (e.g. zip slip) is a client error, an i/o failure is a server error
fn zip_error_status(error: ZipError) -> Status {
    match error {
        ZipError::Io(e) => {
            log::error!("Zip extraction failed: {}", e);
            Status::InternalServerError
        }
        e => {
            log::warn!("Zip archive rejected: {}", e);
            Status::BadRequest
        }
    }
}

// Catches the uploads refused by the routes above (bad token, body that is not multipart/form-data).
// The body is read and dropped before answering: if the server answers without reading it, it
// closes the connection while the client is still sending, and a reverse proxy like nginx
// turns that into a 502 instead of the real status
#[post("/<_..>", data = "<data>", rank = 10)]
pub async fn reject_upload(auth: Option<IsAuth>, data: Data<'_>, limits: &Limits) -> Status {
    let limit = limits.get("data-form").unwrap_or(2.mebibytes());
    let _ = data.open(limit).stream_to(rocket::tokio::io::sink()).await;

    match auth {
        None => Status::Unauthorized,
        Some(_) => Status::UnsupportedMediaType,
    }
}
