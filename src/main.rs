
#[macro_use] extern crate rocket;

use std::collections::HashMap;
use std::io;
use std::path::Path;
use rocket::Data;
use rocket::data::ToByteUnit;
use rocket::http::uri::Absolute;
use rocket::response::content::RawText;
use rocket::tokio::fs::File;

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

#[post("/<file_name>", data = "<file>")]
async fn post_asset(file_name:String,file: Data<'_>)  -> io::Result<String> {

    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    let full_path = Path::new(root).join(&file_name);
    file.open(128.kibibytes()).into_file(full_path).await?;
    Ok(uri!(HOST ,get_asset(file_name)).to_string())
}

#[launch]
fn rocket() -> _ {
    
    rocket::build()
        .mount("/", routes![index])
        .mount("/assets", routes![get_asset])
        .mount("/assets", routes![post_asset])
}