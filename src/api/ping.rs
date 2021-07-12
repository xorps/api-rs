use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::get;

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Ping {
    success: bool
}

#[get("/api/ping")]
pub fn ping() -> Json<Ping> {
    Json(Ping {success: true})
}