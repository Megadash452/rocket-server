use nonempty::NonEmpty;
use serde_json::Value;
use std::io::{BufReader, BufRead, Write};
use serde::Deserialize;
use thiserror::Error;
use super::*;
use crate::components::osts as components;

pub static ALBUMS_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./routes/osts/albums/"));
static COVER_EXPORTS_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./target/song-covers/"));
static SKIP_EXPORT_PATH: Lazy<PathBuf> = Lazy::new(|| COVER_EXPORTS_PATH.join("skip-cover-export.txt"));


#[derive(PartialEq, Eq)]
// TODO: in album page, have a player for each song next to the entry
// ALso have equalizer animation when song plays
// User can upload song
pub struct AlbumInfo {
    pub name: String,
    /// A path relative to the server root.
    pub cover_path: Option<PathBuf>,
    pub artists: Option<NonEmpty<String>>,
    pub remixes: Option<NonEmpty<String>>,
    pub release_year: Option<u32>,
    pub dir_name: String,
    pub size: u32,
    pub complete: bool
}
impl AlbumInfo {
    pub fn find_cover_file(album_dir_name: &str) -> Option<PathBuf> {
        find_files_start(ALBUMS_PATH.join(album_dir_name), THUMB_NAME, true).into_iter().next()
    }
}
impl FromDir for AlbumInfo {
    type Error = AlbumReadError;

    fn read_dir(path: &Path) -> Result<Self, Self::Error> {
        #[derive(Deserialize)]
        struct AlbumInfoJson {
            name: String,
            artist: Option<String>,
            artists: Option<NonEmpty<String>>,
            remixes: Option<NonEmpty<String>>,
            release_year: Option<u32>,
            complete: Option<bool>
        }

        let mut thumbnail_path = None;
        let mut size = 0;
        let contents = path.read_dir()?
            .filter_map(Result::ok)
            .filter(|entry| entry.metadata().ok().is_some_and(|m| m.is_file()));

        let info = serde_json::from_str::<AlbumInfoJson>(
            &std::fs::read_to_string(path.join(INFO_FILE_NAME))?
        )?;

        for file in contents {
            let file_name = file.file_name();
            let file_name = file_name.to_string_lossy();

            if file_name.starts_with(THUMB_NAME) {
                thumbnail_path = Some(path.join(file.file_name()));
                continue;
            }
            if file_name != INFO_FILE_NAME {
                size += 1;
            }
        }

        Ok(Self {
            name: info.name,
            cover_path: thumbnail_path,
            artists: match (info.artist, info.artists) {
                (Some(_), Some(_)) => return Err(AlbumReadError::ArtistFields),
                (Some(artist), None) => Some(NonEmpty::new(artist)),
                (None, artists) => artists,
            },
            release_year: info.release_year,
            remixes: info.remixes,
            dir_name: path.file_name().unwrap().to_string_lossy().to_string(),
            size,
            complete: info.complete.is_some_and(|c| c)
        })
    }
}
impl_ord!(AlbumInfo, name);

#[derive(Debug, Error)]
pub enum AlbumReadError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Deserializing error: {0}")]
    De(#[from] serde_json::Error),
    #[error("Can only have either \"artist\" or \"artists\" fields")]
    ArtistFields
}
impl_error_response!(AlbumReadError);


#[derive(PartialEq, Eq)]
pub struct SongInfo {
    pub title: String,
    pub cover: SongCover,
    pub artists: Option<NonEmpty<String>>,
    pub release_year: Option<u32>,
    pub track_num: Option<u32>,
    pub file_name: String,
    pub album_dir_name: String,
    pub length: String
}
impl FromFile for SongInfo {
    type Error = SongReadError;
    
    fn filter_file(file: &DirEntry) -> bool {
        let file = file.file_name();
        file != INFO_FILE_NAME
        && !file.to_string_lossy().starts_with(THUMB_NAME)
    }
    fn read_file(path: &Path) -> Result<Self, Self::Error> {
        let album_dir_name = path.parent().unwrap().file_name().unwrap().to_string_lossy().to_string();
        let mut command = Command::new("./audio-tag.py");
        command.arg("--json")
            .arg(&path);

        // Find if Song uses album's cover
        let use_album_cover = match std::fs::File::open(&*SKIP_EXPORT_PATH) {
            Ok(file) => {
                let path = path.to_string_lossy();
                BufReader::new(file).lines()
                    .filter_map(|line| line.ok())
                    .find(|line| line == &path)
                    .is_some()
            },
            Err(_) => false
        };

        // Do not export Song's Cover if it uses Album's cover
        if !use_album_cover {
            command.arg("--update-covers")
                .arg("--export-covers-dir").arg(&*COVER_EXPORTS_PATH);
        };

        let output = command.output()
            .map_err(|err| SongReadError::Command(err))?;
        if !output.status.success() {
            return Err(SongReadError::AudioTag(command_output(output.stderr)))
        }

        // Don't waste time looking for exported files that don't exist
        let cover = if use_album_cover {
            SongCover::UseAlbum
        } else {
            // Find if a Cover Images was exported (if any)
            let mut exports = find_files_start(
                &*COVER_EXPORTS_PATH,
                &path.file_name().unwrap().to_string_lossy().to_string(),
                true
            ).into_iter();
            // Note: Song could have multiple covers
            match exports.next() {
                Some(cover) => match AlbumInfo::find_cover_file(&album_dir_name) {
                    // If cover is the same as Album's, use that one instead
                    Some(album_cover) =>
                        if Command::new("cmp")
                            .arg("--silent")
                            .arg(&cover)
                            .arg(&album_cover)
                            .status()
                            .is_ok_and(|is_eq| is_eq.success())
                        {
                            // Exported cover is the same image as Album Cover.
                            // Delete it and use Album Cover instead.
                            std::fs::remove_file(&cover).unwrap_or_default();
                            // Mark file to skip album export next time
                            let mut file = std::fs::OpenOptions::new()
                                .write(true)
                                .append(true)
                                .create(true)
                                .open(&*SKIP_EXPORT_PATH)
                                .expect("Cant open skip exports list");
                            writeln!(file, "{}", path.display()).expect("Can't write skip export");
                            SongCover::UseAlbum
                        } else {
                            // Cover is not the same as Album's
                            SongCover::Some(cover)
                        },
                    // Album has no cover
                    None => SongCover::Some(cover)
                },
                // There is no cover for this Song
                None => SongCover::None
            }
        };

        let mut json = serde_json::from_str::<serde_json::Map<String, Value>>(&command_output(output.stdout))
            .map_err(|err| SongReadError::Json(err))?;

        Ok(Self {
            title: match json.remove("title") {
                Some(Value::String(title)) => title,
                _ => path.with_extension("").file_name().unwrap().to_string_lossy().to_string()
            },
            artists: match json.remove("artist") {
                Some(Value::String(artist)) => Some(
                    NonEmpty::collect(
                        artist.split(',').map(|artist| artist.trim().to_string())
                    )
                    .unwrap_or(NonEmpty::new(artist))
                ),
                _ => None
            },
            release_year: match json.remove("release-year") {
                Some(Value::Number(year)) if year.is_u64() => Some(year.as_u64().unwrap() as u32),
                _ => None
            },
            track_num: match json.remove("track-number") {
                Some(Value::String(track_num)) =>
                    match track_num.split_once('/') {
                        Some((num, _)) => num,
                        None => track_num.as_str()
                    }
                    .parse().ok(),
                _ => None
            },
            length: match json.remove("length") {
                Some(Value::Number(length)) if length.is_u64() => helpers::display_duration(length.as_u64().unwrap()),
                _ => return Err(SongReadError::NoLength)
            },
            cover,
            file_name: path.file_name().unwrap().to_string_lossy().to_string(),
            album_dir_name,
        })
    }
}
impl_ord!(SongInfo, title);

#[derive(Debug, Error)]
pub enum SongReadError {
    #[error("Cannot run audio-tag.py: {0:?}")]
    Command(io::Error),
    #[error("audio-tag.py exited with error: {0}")]
    AudioTag(String),
    #[error("Could not parse JSON: {0:?}")]
    Json(serde_json::Error),
    #[error("Output does not contain Song's length")]
    NoLength
}
impl_error_response!(SongReadError);

#[derive(Debug, PartialEq, Eq)]
pub enum SongCover {
    Some(PathBuf),
    UseAlbum,
    None
}


#[get("/")]
fn index() -> Redirect {
    Redirect::to(uri!("/osts/albums"))
}
#[get("/albums")]
fn albums(user: Option<auth::User>) -> Html<TextStream![String]> {
    Html(TextStream(render_component::<components::AlbumBrowser>(user.into())))
}

#[get("/albums/<album_dir_name>")]
fn view_album(user: Option<auth::User>, album_dir_name: String) -> Result<Html<TextStream![String]>, AlbumReadError> {
    Ok(Html(TextStream(render_component::<components::Album>(components::AlbumProps {
        user: user.into(),
        album: AlbumInfo::read_dir(&ALBUMS_PATH.join(album_dir_name))?
    }))))
}

#[get("/albums/<album_dir_name>/<song_file_name>", format = "text/html")]
fn view_song(user: Option<auth::User>, album_dir_name: String, song_file_name: String) -> Result<Html<TextStream![String]>, SongReadError> {
    Ok(Html(TextStream(render_component::<components::Song>(components::SongProps {
        user: user.into(),
        song: SongInfo::read_file(&ALBUMS_PATH.join(album_dir_name).join(song_file_name))?
    }))))
}
#[get("/albums/<album_dir_name>/<song_file_name>", format = "audio/webm", rank = 1)]
async fn song_file(album_dir_name: String, song_file_name: String) -> io::Result<File> {
    File::open(ALBUMS_PATH.join(album_dir_name).join(song_file_name)).await
}

pub fn routes() -> Vec<Route> {
    routes![index, albums, view_album, view_song, song_file]
}
