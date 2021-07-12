use rocket::tokio::spawn;
use std::collections::HashSet;
use futures::future::try_join_all;
use redis::AsyncCommands;
use super::model::{APIResponse, SortBy, Direction, Post};

/// TODO: maybe use a total ordered crate
/// but this is better than an unwrap
fn f64_compare(a: f64, b: f64) -> std::cmp::Ordering {
    if let Some(ordering) = a.partial_cmp(&b) { return ordering }
    eprintln!("f64_compare failed: {} {}", a, b);
    std::cmp::Ordering::Less
}

pub fn redis_key(tags: &Vec<String>, sort_by: SortBy, direction: Direction) -> String {
    let tags = tags.join(",");
    let sort_by = format!("{}", sort_by);
    let direction = format!("{}", direction);
    [tags, sort_by, direction].join("&")
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error(transparent)]
    RedisError(#[from] redis::RedisError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    JoinError(#[from] rocket::tokio::task::JoinError)
}

async fn try_read_cache(r: Option<redis::aio::ConnectionManager>, tag: &str) -> Result<Option<APIResponse>, Error> {
    let mut r = if let Some(r) = r { r } else { return Ok(None) };
    let cache: Option<String> = r.get(tag).await?;
    let cache = if let Some(cache) = cache { cache } else { return Ok(None) };
    Ok(serde_json::from_str(&cache)?)
}

async fn try_write_cache(r: Option<redis::aio::ConnectionManager>, tag: &str, value: &APIResponse) -> Result<(), Error> {
    let mut r = if let Some(r) = r { r } else { return Ok(()) };
    let value = serde_json::to_string(value)?;
    r.set(tag, value).await?;
    Ok(())
}

async fn fetch_post(r: Option<redis::aio::ConnectionManager>, tag: String) -> Result<APIResponse, Error> {
    if let Some(cache) = try_read_cache(r.clone(), &tag).await? { return Ok(cache) }
    let url = format!("https://api.hatchways.io/assessment/blog/posts?tag={}", tag);
    let response = reqwest::get(url).await?.json::<APIResponse>().await?;
    try_write_cache(r, &tag, &response).await?;
    Ok(response)
}

pub async fn fetch_all(r: Option<redis::aio::ConnectionManager>, tags: Vec<String>, sort_by: SortBy, direction: Direction) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
    async fn go(r: Option<redis::aio::ConnectionManager>, tag: String) -> Result<APIResponse, Error> {
        Ok(spawn(fetch_post(r, tag)).await??)
    }
    let futures = tags.into_iter().map(|tag| go(r.clone(), tag));
    let posts: Vec<Post> = try_join_all(futures).await?.into_iter().map(|r| r.posts).flatten().collect();
    // dedup posts here
    let posts: HashSet<Post> = posts.into_iter().collect();
    // .. back to vec
    let mut posts: Vec<Post> = posts.into_iter().collect();
    // now lets sort it
    posts.sort_by(|a, b| match sort_by {
        SortBy::Id => a.id.cmp(&b.id),
        SortBy::Likes => a.likes.cmp(&b.likes),
        SortBy::Popularity => f64_compare(a.popularity, b.popularity),
        SortBy::Reads => a.reads.cmp(&b.reads),
    });
    // handle direction
    let posts = match direction {
        Direction::Asc => posts,
        Direction::Desc => { posts.reverse(); posts }
    };
    Ok(posts)
}