mod api;
mod cache;
#[cfg(test)] mod tests;

#[rocket::launch]
pub fn rocket() -> _ {
    rocket::build().attach(cache::cache()).mount("/", api::routes())
}
