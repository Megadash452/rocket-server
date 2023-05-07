pub mod osts;
pub mod games;

use std::{path::Path, fs::DirEntry, fmt::Display};
use rocket::{
    Route,
    response::content::RawHtml as Html
};
use super::*;

pub static INFO_FILE_NAME: &str = "info.json";
pub static THUMB_NAME: &str = "thumbnail";


/// Represents the URL of a file in `./routes` that is given to the client.
#[derive(Debug)]
pub struct Url(PathBuf);
impl Url {
    /// Converts a **path** (absolute or relative) into a [`Url`].
    /// Removes all components of the Path *up to and including* the `routes` root.
    pub fn new(path: impl AsRef<Path>) -> Self {
        use std::path::Component;
        Self (
            Some(Component::RootDir).into_iter().chain(
                path.as_ref().components()
                    .skip_while(|comp| comp.as_os_str() != "routes")
                    .skip(1)
            )
            .collect::<PathBuf>()
        )
    }
}
impl<P: AsRef<Path>> From<P> for Url {
    #[inline]
    fn from(path: P) -> Self {
        Self::new(path)
    }
}
impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.display().to_string())
    }
}

/// Tries to read all *subdirectories* in **path** and initialize [`T`]s from the info in each *subdirectory.
/// The directories that couldn't be read into [`T`]s are put in the **error Vec**,
/// along with the [`T::Error`] itself.
pub fn read_all_dirs<T: FromDir + Ord>(dir: &Path) -> (Vec<T>, Vec<(String, T::Error)>) {
    let mut errors = Vec::new();
    let mut items = Vec::new();
    
    for entry in std::fs::read_dir(dir)
        .expect("Can't read dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.metadata().ok().is_some_and(|m| m.is_dir() || m.is_symlink()))
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
    /// Returns [`false`] if file should be skipped.
    fn filter_file(file: &DirEntry) -> bool;
}

#[macro_export]
macro_rules! impl_ord {
    ($st:path, $field:tt) => {
        impl PartialOrd for $st {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.$field.partial_cmp(&other.$field)
            }
        }
        impl Ord for $st {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.$field.cmp(&other.$field)
            }
        }
    };
}
