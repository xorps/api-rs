mod model;
mod util;

use rocket::http::Status;
use rocket::{get, State};
use rocket::response::Debug;
use model::{PostsQuery, PostsResponse};
use util::{fetch_all, redis_key};
use redis::AsyncCommands;

type Error = Debug<Box<dyn std::error::Error>>;

async fn try_read_cache(r: Option<redis::aio::ConnectionManager>, key: &str) -> Result<Option<String>, Error> {
    let mut r = if let Some(r) = r { r } else { return Ok(None) };
    let cache: Option<String> = r.get(key).await.map_err(|e| Debug(e.into()))?;
    Ok(cache)
}

async fn try_write_cache(r: Option<redis::aio::ConnectionManager>, key: &str, value: &str) -> Result<(), Error> {
    let mut r = if let Some(r) = r { r } else { return Ok(()) };
    r.set(key, value).await.map_err(|e| Debug(e.into()))?;
    Ok(())
}

#[get("/api/posts?<query..>")]
pub async fn posts(r: &State<Option<redis::aio::ConnectionManager>>, query: PostsQuery<'_>) -> Result<(Status, String), Error> {
    let r = r.inner();
    let (tags, sort_by, direction) = match query.validate() {
        Err(msg) => {
            let msg = PostsResponse::Error { error: msg.into() };
            let msg = serde_json::to_string(&msg).map_err(|e| Debug(e.into()))?;
            return Ok((Status::BadRequest, msg))
        },
        Ok(val) => val,
    };
    let key = redis_key(&tags, sort_by, direction);
    if let Some(cache) = try_read_cache(r.clone(), &key).await? { return Ok((Status::Ok, cache)) }
    let posts = fetch_all(r.clone(), tags, sort_by, direction).await.map_err(|e| Debug(e.into()))?;
    let value = serde_json::to_string(&PostsResponse::Success {posts}).map_err(|e| Debug(e.into()))?;
    let _ = try_write_cache(r.clone(), &key, &value).await?;
    Ok((Status::Ok, value))
}