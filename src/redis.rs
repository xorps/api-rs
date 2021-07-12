use rocket::fairing::{AdHoc, Fairing};
use std::env::VarError;

enum Open {
    Ok(redis::aio::ConnectionManager),
    Ignore,
}

async fn open() -> Result<Open, Box<dyn std::error::Error>> {
    let redis_url = match std::env::var("REDIS_URL") {
        Ok(url) => url,
        Err(VarError::NotPresent) => return Ok(Open::Ignore),
        Err(e) => return Err(e.into()),
    };
    let client = redis::Client::open(redis_url)?;
    let mgr = redis::aio::ConnectionManager::new(client).await?;
    Ok(Open::Ok(mgr))
}

pub fn redis() -> impl Fairing {
    AdHoc::try_on_ignite("redis", |rocket| async {
        match open().await {
            Ok(Open::Ok(con)) => { println!("Using redis"); Ok(rocket.manage(Some(con))) },
            Ok(Open::Ignore) => { println!("Not using redis"); Ok(rocket.manage::<Option<redis::aio::ConnectionManager>>(None)) },
            Err(e) => { eprintln!("Error caught on redis ignite: {}", e); Err(rocket) },
        }
    })
}