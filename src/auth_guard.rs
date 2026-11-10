use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use subtle::ConstantTimeEq;
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

        let token = request.headers()
            .get_one("Authorization")
            .and_then(|header| header.strip_prefix("Bearer "))
            .unwrap_or("");

        // Constant-time comparison so the response time does not leak how much of the token matched
        let valid = config.api_tokens.iter()
            .any(|expected| bool::from(expected.as_bytes().ct_eq(token.as_bytes())));

        // Forward instead of Error: the request then reaches upload::reject_upload, which reads
        // the body before answering 401 (see there why)
        if token.is_empty() || !valid {
            return Outcome::Forward(Status::Unauthorized);
        }

        Outcome::Success(IsAuth)
    }
}