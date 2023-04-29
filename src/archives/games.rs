use serde::Deserialize;
use thiserror::Error;
use super::*;
use crate::components::games as components;

pub static GAMES_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./routes/games/"));


#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct GameInfo {
    pub title: String,
    pub publisher: String,
    pub genre: String,
    pub release_year: u32,
    // Non-empty array
    pub platforms: Vec<String>,
    pub store_urls: Option<Vec<String>>,
    pub ost_dir_name: Option<String>,
    #[serde(skip)]
    pub dir_name: String,
    #[serde(skip)]
    pub thumbnail_file_name: String
}
impl GameInfo {
    pub fn store_name(mut url: &str) -> &'static str {
        url = url.split_once("://").map(|(_, url)| url).unwrap_or(url);
        url = url.split_once('/').map(|(domain, _)| domain).unwrap_or(url);

        match url {
            "store.steampowered.com" => "Steam",
            "store.epicgames.com" => "Epic Games",
            _ => "Unknown Store"
        }
    }
}
impl FromDir for GameInfo {
    type Error = GameReadError;

    fn read_dir(path: &Path) -> Result<Self, Self::Error> {
        let info = serde_json::from_str::<Self>(
            &std::fs::read_to_string(path.join(INFO_FILE_NAME))
                .map_err(|err| GameReadError::NoInfo(err))?
        )?;
        let thumbnail_file_name = helpers::find_files_start(path, THUMB_NAME, true)
            .pop()
            .ok_or(GameReadError::NoThumbnail)?
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        if info.platforms.is_empty() {
            return Err(GameReadError::NoPlatform)
        }

        Ok(Self {
            dir_name: path.file_name().unwrap().to_string_lossy().to_string(),
            thumbnail_file_name,
            ..info
        })
    }
}
impl_ord!(GameInfo, title);

#[derive(Debug, Error)]
pub enum GameReadError {
    #[error("Can't read {INFO_FILE_NAME}: {0}")]
    NoInfo(io::Error),
    #[error("Deserializing error: {0}")]
    De(#[from] serde_json::Error),
    #[error("Game must have at least 1 platform")]
    NoPlatform,
    #[error("Game has no thumbnail image")]
    NoThumbnail
}


#[get("/")]
fn index(user: Option<auth::User>) -> Html<TextStream![String]> {
    Html(TextStream(render_component::<components::GamesBrowser>(user.into())))
}
#[get("/<game>", rank=1)]
fn game(user: Option<auth::User>, game: String) -> Result<Html<TextStream![String]>, String> {
    let game = GameInfo::read_dir(&GAMES_PATH.join(game))
        .map_err(|err| err.to_string())?;
    Ok(Html(TextStream(render_component::<components::Game>(components::GameProps {
        user: user.into(),
        game
    }))))
}

#[get("/<game>/<first>/<rest..>")]
fn file(game: String, first: String, rest: PathBuf) -> io::Result<std::fs::File> {
    if rest.as_os_str().is_empty() {
        std::fs::File::open(GAMES_PATH.join(game).join(first))
    } else {
        std::fs::File::open(GAMES_PATH.join(game).join(first).join(rest))
    }
}

pub fn routes() -> Vec<Route> {
    routes![index, game, file]
}
