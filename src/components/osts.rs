use std::path::PathBuf;
use yew::prelude::*;
use super::{Document, UserInfo, Icon, item_error};
use crate::helpers::display_separated;
use crate::archives::{ Url, osts::{AlbumInfo, SongInfo, SongCover, ALBUMS_PATH}};
pub use crate::components::UserInfo as AlbumBrowserProps;


#[function_component]
pub fn AlbumBrowser(props: &UserInfo) -> Html {
    html! {
        <Document title="Albums" header={ props.clone() }>
            <link rel="stylesheet" href="/osts/style.css"/>
            <h1>{ "Soundtracks" }</h1>
            <ul id="albums">{{
                let (albums, errors) = crate::archives::read_all_dirs::<AlbumInfo>(&*ALBUMS_PATH);

                errors.into_iter()
                    .map(|(dir_name, error)| item_error(dir_name, error.to_string()))
                    .chain(albums.into_iter()
                        .map(album_browser_item))
                    .collect::<Html>()
            }}</ul>
        </Document>
    }
}
fn album_browser_item(album: AlbumInfo) -> Html {
    html! {
        <li class="item album-item horizontal-wrapper">
            <a class="horizontal-wrapper" href={ Url::new("/osts/albums/").join(album.dir_name) }>
                <div class="thumbnail">{ album_cover(&album.cover_path) }</div>

                <div class="title-wrapper">
                    if let Some(artists) = album.artists {
                        <span class="name">{ album.name }</span>
                        <span class="artists-wrapper">{"By "}<span class="artists">{ display_separated(&artists, ", ") }</span></span>
                    } else {
                        <span class="name full">{ album.name }</span>
                    }
                </div>
            </a>

            <div class="more vertical-wrapper">
                if let Some(year) = album.release_year {
                    <span class="release-year">{ "Released on " }<span class="year">{ year }</span></span>
                }
                <span class="album-size"><span class="size">{ album.size }</span>{ " Songs" }</span>

                if !album.complete {
                    <span class="incomplete">{ "Album is Incomplete" }</span>
                }
            </div>
        </li>
    }
}


#[derive(Properties, PartialEq, Eq)]
pub struct AlbumProps {
    pub user: UserInfo,
    pub album: AlbumInfo
}
#[function_component]
pub fn Album(props: &AlbumProps) -> Html {
    html! {
        <Document title={ props.album.name.clone() } header={ props.user.clone() }>
            <link rel="stylesheet" href="/osts/style.css"/>
            <h1>{ "Soundtracks" }</h1>

            <div id="thumbnail" class="thumbnail">{ album_cover(&props.album.cover_path) }</div>
            <h1 id="name">{ props.album.name.clone() }</h1>
            if let Some(artists) = &props.album.artists {
                <h3 id="artists">{"By "}<span class="artists">{ display_separated(artists, ", ") }</span></h3>
            }
            <h4 id="more">
                if let Some(year) = props.album.release_year {
                    <span id="release-year" class="release-year">{ "Released on " }<span class="year">{ year }</span></span>
                }
                <span id="album-size" class="album-size"><span class="size">{ props.album.size }</span>{ " Songs" }</span>
                if !props.album.complete {
                    <span class="incomplete">{ "Incomplete" }</span>
                }
                if let Some(remixes) = &props.album.remixes {
                    <div>
                        <span>{"Remixes: "}</span>
                        <ul id="remixes">{
                            remixes.iter()
                                .map(|dir_name| html!{
                                    <li><a href={ Url::new("/osts/albums").join(dir_name) }>{ dir_name }</a></li>
                                })
                                .collect::<Html>()
                        }</ul>
                    </div>
                }
            </h4>

            <ul id="songs">{{
                let (songs, errors) = crate::archives::read_all_files::<SongInfo>(&*ALBUMS_PATH.join(&props.album.dir_name));

                errors.into_iter()
                    .map(|(file_name, error)| item_error(file_name, error.to_string()))
                    .chain(songs.into_iter()
                        .map(song_item))
                    .collect::<Html>()
            }}</ul>
        </Document>
    }
}
fn song_item(song: SongInfo) -> Html {
    html! {
        <li class="item song-item horizontal-wrapper">
            <a class="horizontal-wrapper" href={ PathBuf::from("/osts/albums/").join(&song.album_dir_name).join(&song.file_name).display().to_string() }>
                <div class="thumbnail">{ song_cover(&song) }</div>
                
                if let Some(num) = song.track_num {
                    <span class="track-number"><span class="num">{ num }</span>{ ": " }</span>
                }
                <div class="title-wrapper">
                    if let Some(artists) = song.artists {
                        <span class="name">{ song.title }</span>
                        <span class="artists-wrapper">{"By "}<span class="artists">{ display_separated(&artists, ", ") }</span></span>
                    } else {
                        <span class="name full">{ song.title }</span>
                    }
                </div>
            </a>

            <div class="more vertical-wrapper">
                if let Some(year) = song.release_year {
                    <span class="release-year">{ "Released on " }<span class="year">{ year }</span></span>
                }
                <span class="song-length">{ song.length }</span>
            </div>
        </li>
    }
}


#[derive(Properties, PartialEq, Eq)]
pub struct SongProps {
    pub user: UserInfo,
    pub song: SongInfo
}
#[function_component]
pub fn Song(props: &SongProps) -> Html {
    html! {
        <Document title={ props.song.title.clone() } header={props.user.clone()}>
            <link rel="stylesheet" href="/osts/style.css"/>
            <h1>{ "Soundtracks" }</h1>

            <div id="thumbnail" class="thumbnail">{ song_cover(&props.song) }</div>
            <h1 id="name">{ &props.song.title }</h1>
            if let Some(artists) = &props.song.artists {
                <h3 id="artists">{"By "}<span class="artists">{ display_separated(artists, ", ") }</span></h3>
            }
            <h4 id="more">
                if let Some(num) = props.song.track_num {
                    <span class="track-number">{ "#" }<span class="num">{ num }</span></span>
                }
                if let Some(year) = props.song.release_year {
                    <span id="release-year" class="release-year">{ "Released on " }<span class="year">{ year }</span></span>
                }
                <span id="song-length" class="song-length"><span class="length">{ &props.song.length }</span></span>
            </h4>
            <audio controls=true>
                <source src={ Url::new("/osts/albums").join(&props.song.album_dir_name).join(&props.song.file_name) }/>
            </audio>
        </Document> 
    }
}


fn album_cover(path: &Option<PathBuf>) -> Html {
    match path {
        Some(path) => html! { <img src={ Url::new("/files").join(path) }/> },
        None => html! { <Icon name="default-album"/> }
    }
}
fn song_cover(song: &SongInfo) -> Html {
    match (&song.cover, AlbumInfo::find_cover_file(&song.album_dir_name)) {
        // Song has its own cover
        (SongCover::Some(ref path), _)
        // Song uses Album's cover
        | (SongCover::UseAlbum, Some(ref path)) => html!{ <img src={ Url::new("/files").join(path) }/> },
        // No cover exists
        _ => html! { <Icon name="default-song"/> }
    }
}
