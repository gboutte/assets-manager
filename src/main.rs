
#[macro_use] extern crate rocket;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use filesize::file_real_size;
use rocket::Data;
use rocket::data::ToByteUnit;
use rocket::fs::TempFile;
use rocket::http::Method;
use rocket::http::uri::Absolute;
use rocket::response::content::RawText;
use rocket::tokio::fs::File;
use rocket_cors::{AllowedHeaders, AllowedOrigins};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

const HOST: Absolute<'static> = uri!("http://localhost:8000");

#[get("/<file_name>")]
async fn get_asset(file_name:String) -> Option<RawText<File>> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    let full_path = Path::new(root).join(file_name);

    println!("fullPath: {:?}", full_path);

    File::open(full_path).await.map(RawText).ok()
}

#[post("/<file_name_buf..>", data = "<file>")]
async fn post_asset(file_name_buf:PathBuf, mut file:TempFile<'_>)  -> io::Result<String> {

    let file_name = file_name_buf.clone().into_os_string().into_string().unwrap();
    let mimetype = file.content_type().unwrap();
    let filesize = file_real_size(file.path().unwrap()).unwrap();
    print!("File {}, type: {}, size: {}",file_name,mimetype,filesize);

    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    let full_path = Path::new(root).join(&file_name);
    
    let prefix = full_path.parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap();
    file.persist_to(full_path).await?;
    
    
    Ok(uri!(HOST ,get_asset(file_name)).to_string())
}

#[launch]
fn rocket() -> _ {

    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    }
        .to_cors().unwrap();

    rocket::build()
        .attach(cors)
        .mount("/", routes![index])
        .mount("/assets", routes![get_asset])
        .mount("/assets", routes![post_asset])
}