mod api;
mod redis;
#[cfg(test)] mod tests;

#[rocket::launch]
pub fn rocket() -> _ {
    rocket::build().attach(redis::redis()).mount("/", api::routes())
}
