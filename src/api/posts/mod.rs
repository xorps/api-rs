pub mod model;
mod util;

use rocket::http::Status;
use rocket::{get, State};
use rocket::Request;
use rocket::response::{self, Responder};
use rocket::serde::{Serialize, Deserialize, json::Json};
use model::{PostsQuery, PostsQueryError, PostsResponse};
use util::fetch_all;
use crate::cache::{self, IntoCacheKey, Cache};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    FetchError(#[from] util::Error),
    #[error(transparent)]
    CacheError(#[from] cache::Error),
    #[error(transparent)]
    PostsQueryError(#[from] PostsQueryError),
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ServerError<'a> {
    status: &'a str,
    message: &'a str,
}

impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        match self {
            Self::PostsQueryError(PostsQueryError(error)) => (Status::BadRequest, Json(PostsResponse::Error {error})).respond_to(req),
            err => {
                eprintln!("Internal Server Error: {}", err);
                (Status::InternalServerError, Json(ServerError {status: "error", message: "Internal Server Error"})).respond_to(req)
            }
        }
    }
}

#[get("/api/posts?<query..>")]
pub async fn posts(r: &State<Cache>, query: PostsQuery<'_>) -> Result<(Status, String), Error> {
    let r = r.inner();
    let (tags, sort_by, direction) = query.validate()?;
    let key = (&tags, sort_by, direction).into_cache_key();
    if let Some(cache) = r.try_read(&key).await? { return Ok((Status::Ok, cache)) }
    let posts = fetch_all(r.clone(), tags, sort_by, direction).await?;
    let value = serde_json::to_string(&PostsResponse::Success {posts})?;
    r.try_write(&key, &value).await?;
    Ok((Status::Ok, value))
}