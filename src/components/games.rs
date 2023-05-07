use std::fs::Metadata;
use std::path::{PathBuf, Path};
use std::process::Command;
use yew::prelude::*;
use crate::archives::{THUMB_NAME, INFO_FILE_NAME, Url};
use crate::components::{load_svg, text_file};
use crate::helpers::{display_separated, command_output};
use crate::archives::games::{GameInfo, PlatFile, GAMES_PATH, PLATFORM_PREFIX};
use super::{Document, UserInfo, item_error};


#[function_component]
pub fn GamesBrowser(props: &UserInfo) -> Html {
    html! {
        <Document title="Games" header={props.clone()}>
            <link rel="stylesheet" href="/games/style.css"/>
            <h1>{ "Games" }</h1>
            <ul id="albums">{{
                let (games, errors) = crate::archives::read_all_dirs::<GameInfo>(&*GAMES_PATH);

                errors.into_iter()
                    .map(|(dir_name, error)| item_error(dir_name, error.to_string()))
                    .chain(games.into_iter()
                        .map(games_browser_item))
                    .collect::<Html>()
            }}</ul>
        </Document>
    }
}
fn games_browser_item(game: GameInfo) -> Html {
    let game_url = PathBuf::from("/games/").join(&game.dir_name);
    html! {
        <li class="item horizontal-wrapper">
            <a class="horizontal-wrapper" href={ game_url.display().to_string() }>
                <div class="thumbnail"><img src={ game_url.join(&game.thumbnail_file_name).display().to_string() }/></div>
                
                <div class="title-wrapper">
                    <span class="name">{ game.title }</span>
                    <span class="platforms">{"For "}<span>{ display_separated(game.platforms, ", ") }</span></span>
                </div>
            </a>

            <div class="more vertical-wrapper">
                <span class="genre">{ "Genre: " }<span>{ game.genre }</span></span>
                <span class="publisher">{ "Published by: " }<span>{ game.publisher }</span></span>
                <span class="release-year">{ "Published on " }<span>{ game.release_year }</span></span>
                if let Some(urls) = store_urls(&game.store_urls) {
                    <span class="stores">{ "Get it on " }{ urls }</span>
                }
            </div>
        </li>
    }
}


#[derive(Properties, PartialEq, Eq)]
pub struct GameProps {
    pub user: UserInfo,
    pub game: GameInfo,
}
#[function_component]
pub fn Game(props: &GameProps) -> Html {
    html! {
        <Document title={ props.game.title.clone() } header={props.user.clone()}>
            <link rel="stylesheet" href="/games/style.css"/>
            <h1>{ "Games" }</h1>

            <div id="info" class="horizontal-wrapper">
                <div id="thumbnail" class="thumbnail">
                    <img src={ props.game.url().join(&props.game.thumbnail_file_name).display().to_string() }/>
                </div>
                <div class="vertical-wrapper">
                    <div id="title-wrapper">
                        <h1 id="name"><span>{ &props.game.title }</span></h1>
                        <h3 id="publisher">{ "By " }<span>{ &props.game.publisher }</span></h3>
                    </div>
                    <div id="more">
                        <p id="genre">{ "Genre: " }<span>{ &props.game.genre }</span></p>
                        <p id="release-year">{ "Released on " }<span>{ props.game.release_year }</span></p>
                        <p id="platforms">{ "For " }<span>{ display_separated(&props.game.platforms, ", ") }</span></p>
                        if let Some(urls) = store_urls(&props.game.store_urls) {
                            <p class="stores">{ "Get it on " }<span>{ urls }</span></p>
                        }
                        if let Some(dir_name) = &props.game.ost_dir_name {
                            <p><a id="ost" href={ format!("/osts/albums/{dir_name}") }>{ "Soundtrack" }</a></p>
                        }
                    </div>
                </div>
                <div id="game-download">
                    if let Some(files) = props.game.binaries() {{
                        plat_file("Download", files)
                    }} else {{
                        "No Downloads available"
                    }}
                </div>
            </div>
            <div id="content">
                { readme(&props.game.path()) }
                { misc_content(&props.game) }
                // { misc_dirs(&props.game) }
            </div>
        </Document>
    }
}

fn readme(path: &Path) -> Option<Html> {
    crate::helpers::find_files_start(path, "readme", false)
        .pop()
        .map(|path| html! {
            <details open=true class="readme">
                <summary><span>{ load_svg("info") }{ "README" }<a class="download" href={ Url::from(&path).to_string() }>{ load_svg("download") }</a></span></summary>
                { text_file(&path) }
            </details>
        })
}

/// Returned Vec contains (name, url)
fn store_urls(urls: &Option<Vec<String>>) -> Option<Html> {
    urls.as_ref().map(|urls|
        urls.iter()
            .map(|url| html!{
                <a class="store" href={url.clone()} target="_blank">{ GameInfo::store_name(&url) }</a>
            })
            .intersperse(html!{<>{", "}</>})
            .collect::<Html>()
    )
}


fn misc_content(game: &GameInfo) -> Html {
    game.platformed_files().into_iter()
        .map(|(file_name, files)| plat_file(&file_name, files))
        // Also platformed directories
        // .chain(game.platformed_dirs().iter()
        //     .map(||))
        // Also other non-platformed files and directories
        .chain(game.path().read_dir()
            .expect("Game does not exist")
            .filter_map(Result::ok)
            .filter_map(crate::helpers::resolve_entry)
            .filter(|(path, _)| {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                name != INFO_FILE_NAME
                && !name.starts_with(THUMB_NAME)
                && !name.to_lowercase().starts_with("readme")
                && !name.starts_with(PLATFORM_PREFIX)
            })
            .map(|(path, meta)| fs_content(path, meta))
        )
        .map(|html| html! {
            <li class="item">{ html }</li>
        })
        .collect()
}

/// Render the contents of a *file* or *directoru* (recursively) to [`HTML`].
fn fs_content(path: PathBuf, meta: Metadata) -> Html {
    /// All the files and subdirs of a directory.
    fn dir_content(dir_path: PathBuf) -> impl Iterator<Item = Html> {
        // Look for README.md for this directory
        readme(&dir_path).into_iter().chain(
            // Then all other files
            dir_path.read_dir()
                .expect("Can't go any further")
                .filter_map(Result::ok)
                .filter(|entry| !entry.file_name().to_string_lossy().to_lowercase().starts_with("readme"))
                .filter_map(crate::helpers::resolve_entry)
                .map(|(path, meta)| html! {
                    <li class="item">{ fs_content(path, meta) }</li>
                })
        )
    }

    let name = path.file_name().unwrap().to_string_lossy().to_string();

    if meta.is_file() {
        let mime = Command::new("file")
            .arg("-b")
            .arg("--mime-type")
            .arg(&path)
            .output()
            .ok()
            .map(|out| command_output(out.stdout));

        if mime.is_some_and(|mime| mime == "text/plain") {
            html! {
                <details class="file text">
                    <summary>{ load_svg("text-file") }{ name }</summary>
                    { text_file(&path) }
                </details>
            }
        } else {
            html! { <a class="file" href={ Url::from(path).to_string() }>{ load_svg("file") }{ name }</a> }
        }
    } else { // is_dir
        html! {
            <details class="dir">
                <summary>{ load_svg("folder") }{ name }</summary>
                { dir_content(path).collect::<Html>() }
            </details>
        }
    }
}

/// Render a file that exists for multiple platforms.
fn plat_file(name: &str, mut files: Vec<PlatFile>) -> Html {
    let anchor = |file: PlatFile| html! {
        <a href={ Url::from(file.path).to_string() } platform={file.plat.clone()} arch={file.arch.clone()}>{ load_svg(&file.plat) }</a>
    };

    html! {<>
        { load_svg("file") }
        <span class="name">{ &name }</span>

        // Vec is never empty
        if files.len() == 1 {
            <div class="download single">
                <div class="icon-wrapper">{ load_svg("download") }</div>
                { anchor(files.remove(0)) }
            </div>
        } else {
            <div class="download">
                <span class="icon-wrapper">{ load_svg("download") }</span>
                <span class="platforms">{
                    files.into_iter()
                        .map(anchor)
                        .collect::<Html>()
                }</span>
            </div>
        }
    </>}
}
