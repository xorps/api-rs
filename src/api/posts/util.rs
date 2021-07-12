use rocket::tokio::spawn;
use std::collections::HashSet;
use futures::future::try_join_all;
use super::model::{APIResponse, SortBy, Direction, Post};
use crate::cache::{self, Cache};

/// TODO: maybe use a total ordered crate
/// but this is better than an unwrap
fn f64_compare(a: f64, b: f64) -> std::cmp::Ordering {
    if let Some(ordering) = a.partial_cmp(&b) { return ordering }
    eprintln!("f64_compare failed: {} {}", a, b);
    std::cmp::Ordering::Less
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    CacheError(#[from] cache::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    JoinError(#[from] rocket::tokio::task::JoinError)
}

async fn fetch_post(r: Cache, tag: String) -> Result<APIResponse, Error> {
    if let Some(cache) = r.try_read(&tag).await? { return Ok(serde_json::from_str(&cache)?) }
    let url = format!("https://api.hatchways.io/assessment/blog/posts?tag={}", tag);
    let response = reqwest::get(url).await?.json::<APIResponse>().await?;
    r.try_write(&tag, &serde_json::to_string(&response)?).await?;
    Ok(response)
}

pub async fn fetch_all(r: Cache, tags: Vec<String>, sort_by: SortBy, direction: Direction) -> Result<Vec<Post>, Error> {
    async fn go(r: Cache, tag: String) -> Result<APIResponse, Error> {
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