use std::path::{PathBuf, Path};
use std::process::Command;
use nonempty::NonEmpty;
use rocket::Either;
use yew::prelude::*;
use crate::helpers::{display_separated, command_output};
use crate::archives::{ Url, games::{GameInfo, PlatFile, GAMES_PATH, GameFile}};
use super::{Document, Icon, UserInfo, item_error, text_file};


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
                    <span class="platforms">{"For "}<span>{ display_separated(&game.platforms, ", ") }</span></span>
                </div>
            </a>

            <div class="more vertical-wrapper">
                <span class="genre">{ "Genre: " }<span>{ game.genre }</span></span>
                <span class="publisher">{ "Published by: " }<span>{ game.publisher }</span></span>
                <span class="release-year">{ "Published on " }<span>{ game.release_year }</span></span>
                if let Some(urls) = store_urls(game.store_urls.as_ref()) {
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
/// Rendering this page is a very expensive operation and should be *cached*.
pub fn Game(props: &GameProps) -> Html {
    html! {
        <Document title={ props.game.title.clone() } header={props.user.clone()}>
            <link rel="stylesheet" href="/games/style.css"/>
            <h1>{ "Games" }</h1>

            <div id="info" class="horizontal-wrapper">
                <div id="thumbnail" class="thumbnail">
                    <img src={ props.game.url().join(&props.game.thumbnail_file_name).to_string() }/>
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
                        if let Some(urls) = store_urls(props.game.store_urls.as_ref()) {
                            <p class="stores">{ "Get it on " }<span>{ urls }</span></p>
                        }
                        if let Some(dir_name) = &props.game.ost_dir_name {
                            <p><a id="ost" href={ format!("/osts/albums/{dir_name}") }>{ "Soundtrack" }</a></p>
                        }
                    </div>
                </div>
                <div id="game-download">{
                    match props.game.binaries() {
                        Some(Either::Left(file)) => html! {
                            <a href={ Url::from(file) } class="horizontal-wrapper">
                                <Icon name="file"/>
                                <span class="name">{ "Download" }</span>
                                <div class="download single">
                                    <div class="icon-wrapper"><Icon name="download"/></div>
                                </div>
                            </a>
                        },
                        Some(Either::Right(files)) => plat_file("Download", files),
                        None => html! { "No Downloads available" }
                    }
                }</div>
            </div>
            <div id="content">
                { readme(&props.game.path()) }
                { match props.game.files() {
                    // TODO: sort files before directories
                    Ok(files) => files.map(fs_content).collect::<Html>(),
                    Err(conflict) => item_error("Game files".to_string(), conflict.to_string())
                } }
            </div>
        </Document>
    }
}

fn readme(path: &Path) -> Option<Html> {
    crate::helpers::find_files_start(path, "readme", false)
        .pop()
        .map(|path| html! {
            <details open=true class="readme">
                <summary><span><Icon name="info"/>{ "README" }<a class="download" href={ Url::from(&path).to_string() }><Icon name="download"/></a></span></summary>
                { text_file(&path) }
            </details>
        })
}

/// Returned Vec contains (name, url)
fn store_urls(urls: Option<&NonEmpty<String>>) -> Option<Html> {
    urls.map(|urls|
        urls.iter()
            .map(|url| html!{
                <a class="store" href={url.clone()} target="_blank">{ GameInfo::store_name(&url) }</a>
            })
            .intersperse(html!{<>{", "}</>})
            .collect::<Html>()
    )
}

/// Render the contents of a *file* or *directoru* (recursively) to [`HTML`].
fn fs_content(file: GameFile) -> Html {
    fn is_mime(file: &Path, mime: &str) -> bool {
        Command::new("file")
            .arg("-b")
            .arg("--mime-type")
            .arg(file)
            .output()
            .is_ok_and(|out| mime == command_output(out.stdout))
    }

    let name = file.name();

    html! {
        <li class="item">{ match file {
            GameFile::Dir(path) => html! {
                <details class="dir">
                    <summary><Icon name="folder"/>{ name }</summary>
                    {GameFile::read_dir(&path)
                        .map(fs_content)
                        .collect::<Html>()
                    }
                </details>
            },
            GameFile::NormalFile(path) =>
                if is_mime(&path, "text/plain") {
                    html! {
                        <details class="file text">
                            <summary><Icon name="text-file"/>{ &name }</summary>
                            { text_file(&path) }
                        </details>
                    }
                } else {
                    html! { <a class="file" href={ Url::from(path) }><Icon name="file"/>{ &name }</a> }
                },
            // TODO: show content if file is text/plain, with tabs to switch between platforms
            GameFile::PlatFile(files) => plat_file(&name, files)
        } }</li>
    }
}

/// Render a file that exists for multiple platforms.
fn plat_file(name: &str, files: NonEmpty<PlatFile>) -> Html {
    fn anchor(file: PlatFile) -> Html {
        html! { <a href={ Url::from(file.path) } platform={ file.plat.clone() } arch={ file.arch }><Icon name={file.plat}/></a> }
    }

    html! {<>
        <Icon name="file"/>
        <span class="name">{ name }</span>

        if files.len() == 1 {
            <div class="download single">
                <div class="icon-wrapper"><Icon name="download"/></div>
                { anchor(files.head) }
            </div>
        } else {
            <div class="download">
                <span class="icon-wrapper"><Icon name="download"/></span>
                <span class="platforms">{
                    files.into_iter()
                        .map(anchor)
                        .collect::<Html>()
                }</span>
            </div>
        }
    </>}
}
