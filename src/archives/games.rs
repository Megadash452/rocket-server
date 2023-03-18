use serde::Deserialize;
use thiserror::Error;
use super::*;
use crate::components::games as components;

static GAMES_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./routes/games/"));


#[derive(Deserialize)]
pub struct GameInfo {
    title: String,
    publisher: String,
    genre: String,
    platforms: Vec<String>,
    release_year: u32,
    store_url: Option<String>,
    ost_url: Option<String>,
    #[serde(skip)]
    dir_name: String,
    #[serde(skip)]
    thumbnail_path: PathBuf
}
impl FromDir for GameInfo {
    type Error = GameReadError;

    fn read_dir(path: &Path) -> Result<Self, GameReadError> {
        todo!();
    }
}

#[derive(Debug, Error)]
pub enum GameReadError {
    #[error("Game must have at least 1 platform")]
    MissingPlatform
}



#[get("/")]
fn index(user: Option<auth::User>) -> Html<TextStream![String]> {
    Html(TextStream(render_component::<components::GamesBrowser>(user.into())))
}

pub fn routes() -> Vec<Route> {
    routes![index]
}
