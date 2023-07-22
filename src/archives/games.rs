use std::{
    fs,
    collections::VecDeque,
    io::{Read, Write}
};
use nonempty::NonEmpty;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use super::*;
use crate::components::games as components;

pub static GAMES_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./routes/games/"));
pub static PLATFORM_PREFIX: &str = "plat-";


#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatFile {
    /// Path relative to server root.
    pub path: PathBuf,
    pub plat: String,
    /// Assumes default architecture of system (e.g. Windows and Linux are `x64`).
    pub arch: Option<String>
}

#[derive(Debug)]
pub enum GameFile {
    Dir(PathBuf),
    NormalFile(PathBuf),
    PlatFile(NonEmpty<PlatFile>)
}
impl GameFile {
    /// Returns error if entry is file and can't read it.
    pub fn from_entry(entry: DirEntry) -> io::Result<Self> {
        if entry.file_type()?.is_dir() {
           Ok(GameFile::Dir(entry.path()))
        } else {
            let content = fs::read_to_string(entry.path())?;

            if content.starts_with(GameInfo::NORMAL_FILE_CONTENT) {
                Ok(GameFile::NormalFile(entry.path()))
            } else {
                Ok(GameFile::PlatFile(NonEmpty::collect(
                    content.split('\n')
                        .filter(|s| !s.is_empty())
                        .map(|s| serde_json::from_str::<PlatFile>(s)
                            .expect("Could not deserialize PlatFile")
                        )
                    ).expect("PlatFile can't be empty")))
            }
        }
    }
    
    /// Get the "filename" of the file or directory.
    pub fn name(&self) -> String {
        let path = match self {
            Self::Dir(path) | Self::NormalFile(path) => path,
            Self::PlatFile(plats) => &plats.first().path
            
        };
        path.file_name().unwrap().to_string_lossy().to_string()
    }

    /// Use with path from [`Self::Dir`].
    pub fn read_dir(dir: &Path) -> impl Iterator<Item = Self> {
        dir.read_dir()
            .expect("Could not read GameFile::Dir")
            .map(|r| GameFile::from_entry(r.unwrap())
                .expect("Can't read file to GameFile")
            )
    }
}

#[derive(Debug, Error)]
pub enum Conflict {
    #[error("A directory exists in place of file {file:?}")]
    FileDir {
        /// Path (relative to server root) of the platformed **file** that exists.
        file: PathBuf
    },
    #[error("A normal file exists in place of platformed file {0:?}. Platformed files cannot coexist with normal files.")]
    NormalExists(PathBuf),
    #[error("Found symlink at {0:?}. Symlinks are not allowed within game directory")]
    Symlink(PathBuf)
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
/// An instance of [`Self`] should be obtained by calling [`FromDir::read_dir()`], then use [`Self::binaries()`], [`Self::files()`].
/// A `README` for self and each subdirectory must be obtained separately by [`finding files that start`](crate::helpers::find_files_start) with `"README"` (case-insensitive)
pub struct GameInfo {
    pub title: String,
    pub publisher: String,
    pub genre: String,
    pub release_year: u32,
    pub platforms: NonEmpty<String>,
    pub store_urls: Option<NonEmpty<String>>,
    pub ost_dir_name: Option<String>,
    #[serde(skip)]
    pub dir_name: String,
    #[serde(skip)]
    pub thumbnail_file_name: String
}
impl GameInfo {
    const NORMAL_FILE_CONTENT: &str = "normal";

    pub fn path(&self) -> PathBuf {
        GAMES_PATH.join(&self.dir_name)
    }
    pub fn url(&self) -> Url {
        Url::from(self.path())
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
    pub fn binaries(&self) -> Option<NonEmpty<PlatFile>> {
        // Search the directories that have files specific to a platform (e.g. Windows and Linux)
        NonEmpty::collect(
            self.plat_dirs()
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
        )
    }

    /// TODO: doc
    /// Vec is the content of all the files that immediately belong to ./target/games-files/{game}/
    pub fn files(&self) -> Result<impl Iterator<Item = GameFile>, Conflict> {
        Ok(self.create_file_locations()?
            .read_dir()
            .expect("Can't read into created directory")
            .map(|r| GameFile::from_entry(r.unwrap())
                .expect("Can't read file to GameFile")
            )
        )
    }

    /// Create files and directories in `"./target/games-files/{game}/"` for easier iteration in [`Self::files()`].
    /// Writes the information of each file in the respective directory, using **Breadth-First** traversal of the filesystem.
    /// Returns the path of the directory written to.
    /// 
    /// Format of files in `"./target/games-files/{game}/"`:
    ///  - A file with a single line `normal` is not for a platform (i.e. the actual file is not inside a folder like "plat-linux").
    ///  - A file with a *line-separated* list of [`PlatFile`]s in *JSON* format.
    /// 
    /// Instead of writing to the filesystem, could keep track of nodes in memory, but that was way more complicated than this.
    /// 
    /// ***TODO***: implement this without using the filesystem.
    /// 
    /// Doesn't resolve symlinks, because that't not necessary right now.
    /// Panics if can't write there.
    fn create_file_locations(&self) -> Result<PathBuf, Conflict> {
        let target_path = PathBuf::from(format!("./target/games-files/{}", self.dir_name));
        // TODO: return if no files have changed in self.path() since last call.
        // if target_path.exists() {
        //     return;
        // }

        // Remove target dir. if doesnt exist, its better (hence drop)
        drop(fs::remove_dir_all(&target_path));

        fn read_dir(dir: PathBuf) -> impl Iterator<Item = PathBuf> {
            // Dont return metadata to mitigate ToC/ToU attacks
            dir.read_dir()
                .expect("Can't read into...")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
        }
        /// BFS algorithm for the filesystem. **action** is called on every file (not dir).
        /// **filter** is only applied to the children of **start**.
        /// 
        /// ***WARNING***: Can return symlink. Symlinks should be made into Conflict error.
        fn fs_bfs(start: PathBuf, filter: impl FnMut(&PathBuf) -> bool) -> impl Iterator<Item = PathBuf> {
            let mut iter = vec![];
            let mut queue = VecDeque::new();
            // Note: start can be a symlink that points to a directory
            if !start.is_dir() {
                // iterator is empty if is file
                return iter.into_iter();
            }
            // Step 1: Enqueue the root (Don't, just its children).
            queue.extend(read_dir(start).filter(filter));

            // Step 2: Get next from queue.
            while let Some(path) = queue.pop_front() {
                let meta = path.metadata().expect("Can't get metadata of game file");
                // Step 3: Do something with it.
                if meta.is_symlink() || meta.is_file() {
                    iter.push(path)
                }
                // Step 4: Enqueue its children.
                else if meta.is_dir() {
                    queue.extend(read_dir(path))
                }
            }

            iter.into_iter()
        }

        // Add non-platformed files first.
        for path in fs_bfs(self.path(), |path| {
            // Dont add the platform folders, and exclude game metadata
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            !name.starts_with(PLATFORM_PREFIX)
            && name != INFO_FILE_NAME
            && !name.starts_with(THUMB_NAME)
            && !name.to_lowercase().starts_with("readme")
        }) {
            // Symlinks inside game directory are not allowed.
            if path.is_symlink() {
                return Err(Conflict::Symlink(path));
            }
            // ./target/games-files/{game}/{file...}
            let file_path = target_path.join(path.strip_prefix(self.path()).unwrap());
            fs::create_dir_all(file_path.parent().unwrap())
                .expect(&format!("Can't prepare dir for {file_path:?}"));
            // Write file info
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&file_path)
                .expect(&format!("Can't create file {file_path:?}"));
            writeln!(file, "{}", Self::NORMAL_FILE_CONTENT).expect("Could not write to file");
        }
        
        // Then platformed files, and check for conflict
        for (dir, plat, arch) in self.plat_dirs() {
            for path in fs_bfs(dir.path(), |_| true) {
                // ./target/games-files/{game}/{file...}
                let file_path = target_path.join(path.strip_prefix(self.path().join(dir.file_name())).unwrap());
                // Check file-directory conflicts
                if file_path.is_symlink() {
                    return Err(Conflict::Symlink(path));
                } else if file_path.is_dir() {
                    return Err(Conflict::FileDir { file: path });
                }

                fs::create_dir_all(file_path.parent().unwrap())
                    .expect(&format!("Can't prepare dir for {file_path:?}"));
                let mut file = fs::OpenOptions::new()
                    .read(true)
                    .append(true)
                    .create(true)
                    .open(&file_path)
                    .expect(&format!("Can't open/create file {file_path:?}"));
                // Check conflicts (if the file already exists as normal file)
                let mut buf = [0u8; Self::NORMAL_FILE_CONTENT.len()];
                if file.read_exact(&mut buf).is_ok() && buf == Self::NORMAL_FILE_CONTENT.as_bytes() {
                    return Err(Conflict::NormalExists(path));
                }
                // Can append the path to the file
                serde_json::to_writer(&mut file, &PlatFile { path, plat: plat.clone(), arch: arch.clone() })
                    .expect("Could not write to file");
                writeln!(file, "").expect("Could not write to file");
            }
        }

        Ok(target_path)
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
