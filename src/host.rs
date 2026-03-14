
use rocket::request::{FromRequest, Outcome};

pub struct RequestInfo {
    pub protocol: String,
    pub host: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestInfo {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let protocol = request
            .headers()
            .get_one("X-Forwarded-Proto")
            .unwrap_or("http")
            .to_string();

        let host = request
            .headers()
            .get_one("Host")
            .unwrap_or("localhost")
            .to_string();

        Outcome::Success(RequestInfo { protocol, host })
    }
}