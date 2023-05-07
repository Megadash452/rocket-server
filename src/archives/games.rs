use serde::Deserialize;
use thiserror::Error;
use super::*;
use crate::components::games as components;

pub static GAMES_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./routes/games/"));
pub static PLATFORM_PREFIX: &str = "plat-";


#[derive(Debug)]
pub struct PlatFile {
    /// Path relative to server root.
    pub path: PathBuf,
    pub plat: String,
    /// Assumes default architecture of system (e.g. Windows and Linux are `x64`).
    pub arch: Option<String>
}

pub enum PlatNode {
    File(Vec<PlatFile>),
    Dir(HashMap<String, Self>)
}

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
    pub fn path(&self) -> PathBuf {
        GAMES_PATH.join(&self.dir_name)
    }
    pub fn url(&self) -> PathBuf {
        PathBuf::from("/games/").join(&self.dir_name)
    }

    pub fn store_name(mut url: &str) -> &'static str {
        url = url.split_once("://").map(|(_, url)| url).unwrap_or(url);
        url = url.split_once('/').map(|(domain, _)| domain).unwrap_or(url);

        match url {
            "store.steampowered.com" => "Steam",
            "store.epicgames.com" => "Epic Games",
            _ => "Unknown Store"
        }
    }
    
    /// Gets the paths of the game's binary for each platform.
    /// Returns [`None`] if no file exists with the same base-name as the game's directory.
    /// THe [`Vec`] in [`Some`] is never empty.
    pub fn binaries(&self) -> Option<Vec<PlatFile>> {
        // Search the directories that have files specific to a platform (e.g. Windows and Linux)
        let rtrn = self.plat_dirs()
            .filter_map(|(dir, plat, arch)|{
                let file = dir.path().read_dir()
                    .expect("can't read platform dir")
                    .filter_map(Result::ok)
                    .filter(|entry| entry.metadata().ok().is_some_and(|m| m.is_file()))
                    // The game file should have the same base name as the game's directory.
                    .filter(|entry| entry.path().file_prefix().is_some_and(|base_name| base_name.to_string_lossy() == self.dir_name.as_str()))
                    .next()?;
                Some(PlatFile { path: file.path(), plat, arch })
            })
            .collect::<Vec<_>>();

        if rtrn.is_empty() {
            None
        } else {
            Some(rtrn)
        }
    }
    /// like binaries but for all other files.
    /// [`Vec`] is never empty.
    pub fn platformed_files(&self) -> HashMap<String, Vec<PlatFile>> {
        let mut files: HashMap<String, Vec<PlatFile>> = HashMap::new();

        for (dir, plat, arch) in self.plat_dirs() {
            for file in dir.path()
                .read_dir()
                .expect("can't read platform dir")
                .filter_map(Result::ok)
                .filter(|entry| entry.metadata().ok().is_some_and(|m| m.is_file()))
                // Exclude game binary
                .filter(|entry| entry.path().file_prefix().is_some_and(|base_name| base_name.to_string_lossy() != self.dir_name.as_str()))
            {
                let name = file.file_name().to_string_lossy().to_string();
                let new_file = PlatFile {
                    path: file.path(),
                    plat: plat.clone(),
                    arch: arch.clone()
                };

                if let Some(files) = files.get_mut(&name) {
                    files.push(new_file)
                } else {
                    files.insert(name, vec![new_file]);
                }
            }
        }

        files
    }
    pub fn platformed_content(&self) -> HashMap<String, PlatNode> {
        todo!()
    }

    /// Get the **directories (`0`)** that contain files specific to a **platform (`1`)** or **architecture (`2`)**.
    /// [`DirEntry`] (0) are direcotries with names in this format: `plat-{plat}[-{arch}]`.
    fn plat_dirs(&self) -> impl Iterator<Item = (DirEntry, String, Option<String>)> {
        self.path().read_dir()
            .expect("can't read game dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.metadata().ok().is_some_and(|m| m.is_dir()))
            .filter_map(|entry|
                // Include platform info from this directory
                entry.file_name().to_string_lossy()
                    .strip_prefix(PLATFORM_PREFIX)
                    .map(|rest| match rest.split_once("-") {
                        Some((plat, arch)) => (plat.to_string(), Some(arch.to_string())),
                        None => (rest.to_string(), None )
                    })
                    .map(|(plat, arch)| (entry, plat, arch))
            )
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
