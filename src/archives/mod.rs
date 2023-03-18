pub mod osts;
pub mod games;

use std::{path::Path, fs::DirEntry};
use rocket::{
    Route,
    response::content::RawHtml as Html
};
use super::*;

static INFO_FILE_NAME: &str = "info.json";
static ALBUM_COVER_NAME: &str = "thumbnail";


// #[get("/")]
// pub fn index() -> Redirect {
//     Redirect::to("login?next=archives")
// }

// pub fn routes() -> Vec<Route> {
//     routes![index]
// }

/// Tries to read all *subdirectories* in **path** and initialize [`T`]s from the info in each *subdirectory.
/// The directories that couldn't be read into [`T`]s are put in the **error Vec**,
/// along with the [`T::Error`] itself.
pub fn read_all_dirs<T: FromDir + Ord>(dir: &Path) -> (Vec<T>, Vec<(String, T::Error)>) {
    let mut errors = Vec::new();
    let mut items = Vec::new();
    
    for entry in std::fs::read_dir(dir)
        .expect("Can't read dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.metadata().ok().is_some_and(|m| m.is_dir()))
    {
        match T::read_dir(&entry.path()) {
            Ok(album) => items.push(album),
            Err(error) => errors.push((entry.path().file_name().unwrap().to_string_lossy().to_string(), error))
        }
    }

    items.sort();
    (items, errors)
}

/// Tries to read all *files* in **path** and initialize [`T`]s from each *file*'s content.
/// The files that couldn't be read into [`T`]s are put in the **error Vec**,
/// along with the [`T::Error`] itself.
pub fn read_all_files<T: FromFile + Ord>(file: &Path) -> (Vec<T>, Vec<(String, T::Error)>) {
    let mut errors = Vec::new();
    let mut items = Vec::new();
    
    for entry in std::fs::read_dir(file)
        .expect("Can't read dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.metadata().ok().is_some_and(|m| m.is_file() || m.is_symlink()))
    {
        if !T::filter_file(&entry) {
            continue
        }
        match T::read_file(&entry.path()) {
            Ok(album) => items.push(album),
            Err(error) => errors.push((entry.path().file_name().unwrap().to_string_lossy().to_string(), error))
        }
    }

    items.sort();
    (items, errors)
}

pub trait FromDir: Sized {
    type Error;
    /// path must be a directory, relative to server root.
    fn read_dir(dir: &Path) -> Result<Self, Self::Error>;
}

pub trait FromFile: Sized {
    type Error;
    /// path must be a file, relative to server root.
    fn read_file(file: &Path) -> Result<Self, Self::Error>;
    /// Returns [`true`] if file should be skipped.
    fn filter_file(file: &DirEntry) -> bool;
}
