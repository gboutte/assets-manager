use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use crate::config::Config;

pub struct IsAuth;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for IsAuth {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        // Get config from request
        let config = request.guard::<&rocket::State<Config>>().await;


        let config = match config {
            Outcome::Success(c) => c,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

        let bearer_token = request.headers().get_one("Authorization").unwrap_or("");
        let expected = format!("Bearer {}", config.api_token);
        if bearer_token != expected {
            return Outcome::Error((Status::Unauthorized, ()));
        }

        Outcome::Success(IsAuth)
    }
}