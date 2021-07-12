use parse_display::Display;
use rocket::form::{self, FromForm, FromFormField};
use rocket::serde::{Deserialize, Serialize};

#[derive(Display, Clone, Copy, Debug, FromFormField)]
#[display(style = "lowercase")]
pub enum SortBy {
    Id,
    Reads,
    Likes,
    Popularity,
}

#[derive(Display, Clone, Copy, Debug, FromFormField)]
#[display(style = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, FromForm)]
pub struct PostsQuery<'a> {
    tags: form::Result<'a, &'a str>,
    #[field(name = "sortBy")]
    sort_by: form::Result<'a, SortBy>,
    direction: form::Result<'a, Direction>,
}

/// Converts missing field errors to an Option
/// Unfortunately, in Rocket form::Result<Option<T>> defaults to Ok(None) on invalid values
/// Using Strict mode (Strict<T>) didn't make a difference in this regard
/// so we resort to this
fn check_missing<T>(field: form::Result<'_, T>) -> Result<Option<T>, ()> {
    let err = match field {
        Err(err) => err,
        Ok(val) => return Ok(Some(val)),
    };
    if err.iter().filter(|e| e.kind == form::error::ErrorKind::Missing).count() > 0 { Ok(None) }
    else { Err(()) }
}

/// split comma separated string
fn parse_tags(tags: &str) -> Result<Vec<String>, ()> {
    let tags: Vec<String> = tags.split(',').filter(|s| s.len() > 0).map(|s| s.to_string()).collect();
    if tags.len() > 0 { Ok(tags) }
    else { Err(()) }
}

impl<'a> PostsQuery<'a> {
    pub fn validate(self) -> Result<(Vec<String>, SortBy, Direction), &'static str> {
        let Self {tags, sort_by, direction} = self;
        let tags = tags.map_err(|_| ()).and_then(parse_tags).map_err(|_| "Tags parameter is required")?;
        let sort_by = check_missing(sort_by).map_err(|_| "sortBy parameter is invalid")?.unwrap_or(SortBy::Id);
        let direction = check_missing(direction).map_err(|_| "direction parameter is invalid")?.unwrap_or(Direction::Asc);
        Ok((tags, sort_by, direction))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct APIResponse {
    pub posts: Vec<Post>
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Post {
    pub id: i64,
    pub author: String,
    #[serde(rename = "authorId")]
    pub author_id: i64,
    pub likes: i64,
    pub popularity: f64,
    pub reads: i64,
    pub tags: Vec<String>,
}

/// Assuming Post is unique by its id
/// This allows for cheaper & faster comparison
impl PartialEq for Post {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Post {}

/// Again, assuming we can use a Post's id here.
impl std::hash::Hash for Post {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(untagged)]
pub enum PostsResponse {
    Error { error: String },
    Success { posts: Vec<Post> },
}