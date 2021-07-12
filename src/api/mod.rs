mod ping;
pub mod posts;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![ping::ping, posts::posts]
}