# api-rs
## An example JSON api with Redis and Rocket
Running locally.

The app will use redis for cache if REDIS_URL is present.

Otherwise, it doesn't cache.
```shell
export REDIS_URL="..."
cargo test
cargo run
```
[![Deploy](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy?template=https://github.com/xorps/api-rs)
```shell
heroku create
git push heroku master
heroku open
```