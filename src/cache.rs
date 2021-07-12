use rocket::fairing::{self, AdHoc, Fairing};
use std::env::VarError;
use redis::AsyncCommands;
use crate::api::posts::model::{SortBy, Direction};

pub trait IntoCacheKey {
    fn into_cache_key(&self) -> String;
}

impl IntoCacheKey for (&Vec<String>, SortBy, Direction) {
    fn into_cache_key(&self) -> String {
        format!("{}:{}:{}", self.0.join(","), self.1, self.2)
    }
}

impl IntoCacheKey for &str {
    fn into_cache_key(&self) -> String {
        (*self).into()
    }
}

impl IntoCacheKey for &String {
    fn into_cache_key(&self) -> String {
        (*self).into()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    VarError(#[from] VarError),
    #[error(transparent)]
    RedisError(#[from] redis::RedisError),
}

#[derive(Clone)]
pub struct Cache(Option<redis::aio::ConnectionManager>);

fn get_redis_url() -> Result<Option<String>, VarError> {
    match std::env::var("REDIS_URL") {
        Ok(url) => Ok(Some(url)),
        Err(VarError::NotPresent) => Ok(None),
        Err(e) => Err(e),
    }
}

impl Cache {
    async fn new() -> Result<Self, Error> {
        let url = get_redis_url()?;
        let url = if let Some(url) = url { url } else { return Ok(Self(None)) };
        let client = redis::Client::open(url)?;
        let mgr = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self(Some(mgr)))
    }

    pub async fn try_read(&self, key: impl IntoCacheKey) -> Result<Option<String>, Error> {
        let mut con = if let Self(Some(con)) = self { con.clone() } else { return Ok(None) };
        let value: Option<String> = con.get(key.into_cache_key()).await?;
        Ok(value)
    }

    pub async fn try_write(&self, key: impl IntoCacheKey, value: &str) -> Result<(), Error> {
        let mut con = if let Self(Some(con)) = self { con.clone() } else { return Ok(()) };
        Ok(con.set(key.into_cache_key(), value).await?)
    }
}

async fn ignite(rocket: rocket::Rocket<rocket::Build>) -> fairing::Result {
    match Cache::new().await {
        Ok(c @ Cache(Some(_))) => { println!("Using redis"); Ok(rocket.manage(c)) },
        Ok(c @ Cache(None)) => { println!("Not using redis"); Ok(rocket.manage(c)) },
        Err(e) => { eprintln!("Error caught on redis ignite: {}", e); Err(rocket) },
    }
}

pub fn cache() -> impl Fairing {
    AdHoc::try_on_ignite("redis", ignite)
}