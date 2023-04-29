use std::{
    collections::HashSet,
    path::{PathBuf, Path},
    process::Command,
    fs::DirEntry
};
use yew::prelude::*;
use crate::archives::{INFO_FILE_NAME, THUMB_NAME};
use crate::components::load_svg;
use crate::helpers::{display_separated, command_output, eq_one_of};
use crate::archives::games::{GameInfo, GAMES_PATH};
use super::{Document, UserInfo, item_error};


#[function_component]
pub fn GamesBrowser(props: &UserInfo) -> Html {
    html! {
        <Document title="Games" header={props.clone()}>
            <link rel="stylesheet" href="/games/style.css"/>
            <h1>{ "Games" }</h1>
            <ul id="albums">{{
                let (albums, errors) = crate::archives::read_all_dirs::<GameInfo>(&*GAMES_PATH);

                errors.into_iter()
                    .map(|(dir_name, error)| item_error(dir_name, error.to_string()))
                    .chain(albums.into_iter()
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
                    <img src={ format!("/games/{}/{}", props.game.dir_name, props.game.thumbnail_file_name) }/>
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
                    // TODO:
                </div>
            </div>
            <div id="content">{ misc_contents(&props.game.dir_name) }</div>
        </Document>
    }
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

/// Puts each entry of the *game dir* in the resulting [`HTML`] using `details` and `summary` tags.
/// 
/// Will do different things depening on the type of entry:
/// 1. `File`: gives options ot download file, with different platforms if a subextension matches.
/// 2. `Directory`: Recurses all files and directories.
/// 3. `Symlink`: resolves the link and acts accordingly. Returns *unresolved link* if leads to another link.
fn misc_contents(game_dir_name: &str) -> Html {
    // Store already used filenames to prevent duplicates of platformed files.
    let mut used = HashSet::new();
    let url = PathBuf::from("/games/").join(game_dir_name);

    iter_dir(
        &url,
        &mut used,
        Some(|html| html! { <li class="item">{ html }</li> }),

        GAMES_PATH.join(game_dir_name).read_dir().expect("Game does not exist")
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name();
                name != INFO_FILE_NAME
                && !name.to_string_lossy().to_string().starts_with(THUMB_NAME)
                && name != game_dir_name
            })
    )
    .collect::<Html>()
}

/// Recursively get all files and subdirectories.
fn dir(dir_path: &Path, parent_url: &Path) -> Result<Html, String> {
    // Store already used filenames to prevent duplicates of platformed files.
    let mut used = HashSet::new();
    let name = dir_path.file_name().unwrap().to_string_lossy().to_string();
    let url = parent_url.join(&name);

    Ok(html! {
        <details class="sub-dir">
            <summary>{ load_svg("folder") }{ name }</summary> {
                iter_dir(
                    &url,
                    &mut used,
                    None,
                    dir_path.read_dir()
                        .map_err(|err| format!("Could not go into directory: {err}"))?
                        .filter_map(Result::ok)
                )
                .collect::<Html>()
            }
        </details>
    })
}

/// helper to recurse direcotry
fn iter_dir<'a>(
    url: &'a Path,
    used: &'a mut HashSet<String>,
    transform_html: Option<fn(Html) -> Html>,
    iter: impl Iterator<Item = DirEntry> + 'a
) -> impl Iterator<Item = Html> + 'a
{
    iter
        .filter_map(|entry| entry.metadata().ok().map(|meta| (entry, meta)))
        .map(move |(entry, meta)| {
            let mut name = entry.file_name().to_string_lossy().to_string();

            let res = if meta.is_file() {
                file(&mut name, &entry.path(), &url)
            } else if meta.is_dir() {
                dir(&entry.path(), &url)
            } else { // symlink
                match entry.path().read_link() {
                    Ok(path) => {
                        name = path.file_name().unwrap().to_string_lossy().to_string();

                        if path.is_file() {
                            file(&mut name, &path, &url)
                        } else if path.is_dir() {
                            dir(&path, &url)
                        } else {
                            // Do not resolve again
                            Err("Is Symlink.\nMax depth for symlink resolution is 1".to_string())
                        }
                    },
                    Err(error) => Err(error.to_string())
                }
            };

            (name, res)
        })
        .filter_map(move |(name, res)| {
            match res {
                Ok(html) => if used.insert(name) {
                    // Is not a duplicate
                    Some(match transform_html {
                        Some(f) => f(html),
                        None => html
                    })
                } else {
                    // name was already used; skip this one
                    None
                },
                // If there was an error with the file, use it regardless of if it was repeated or not.
                Err(error) => Some(item_error(name, error))
            }
        })
}

/// If file is platformed (i.e. contains .win, .linux, or .mac subextension) the platform is removed form the file name.
fn file(name: &mut String, path: &Path, parent_url: &Path) -> Result<Html, String> {
    let mime = Command::new("file")
        .arg("-b")
        .arg("--mime-type")
        .arg(path)
        .output()
        .ok()
        .map(|out| command_output(out.stdout));
    
    // Plain text files can be shown directly.
    Ok(if mime.is_some_and(|mime| mime == "text/plain") {
        let content = String::from_utf8_lossy(
            &std::fs::read(path)
                .map_err(|err| format!("Could not read file: {err}"))?
        ).to_string();

        html! {
            <li class="item">
                <details>
                    <summary>{ load_svg("text-file") }{ name }</summary>
                    {{
                        if path.extension().is_some_and(|ext| ext == "md") {
                            // Markdown files can be compiled to HTML.
                            Html::from_html_unchecked(markdown::to_html(&content).into())
                        } else {
                            html! { <code>{ content }</code> }
                        }
                    }}
                </details>
            </li>
        }
    }
    // Binaries/other are downloaded
    else {
        let vars = match find_downloads(path) {
            Some((new_name, vars)) => {
                *name = new_name;
                vars
            },
            None => vec![]
        };

        html! {<>
            { load_svg("file") }
            <span class="name">{ &name }</span>

            if vars.is_empty() {
                // No platform
                <a class="download" href={ parent_url.join(name).display().to_string() }>{ load_svg("download") }</a>
            } else if vars.len() == 1 {
                // Has platform, but not other variants
                <div class="download one">
                    { load_svg("download") }
                    <a href={ parent_url.join(name).display().to_string() }>{ load_svg(&vars[0].0) }</a>
                </div>
            } else {
                // Has other variants
                <div class="downloads">{
                    load_svg("download")
                }{
                    vars.into_iter()
                        .map(|(icon, f_name)| html! {
                            <a href={ parent_url.join(f_name).display().to_string() }>{ load_svg(icon) }</a>
                        })
                        .collect::<Html>()
                }</div>
            }
        </>}
    })
}

/// Find alternative download versions of a binary file.
/// Output contains `(name_without_platform, [(icon_name, file_name of variant)])`
/// Output also includes the original input.
/// 
/// Returns [`None`] if could not recognize a **platform** for this file.
fn find_downloads(file: &Path) -> Option<(String, Vec<(String, String)>)> {
    let name = file.file_name().unwrap().to_string_lossy().to_string();

    let split = name.split_once(".win.")
        .or_else(|| name.split_once(".linux.")
            .or_else(|| name.split_once(".mac."))
        )?;
    let new_name = split.0.to_string() + "." + split.1;
    let (start, end) = (split.0.to_string() + ".", ".".to_string() + split.1);

    Some((new_name,
        // Search for other variants in the parent directory of the file
        crate::helpers::find_files_start(file.parent().unwrap(), &start, true)
            .into_iter()
            .filter_map(|f| {
                let f_name = f.file_name().unwrap().to_string_lossy().to_string();
                let p = f_name.strip_prefix(&start)?.strip_suffix(&end)?;

                if eq_one_of(p, ["win", "linux", "mac"]) {
                    Some((p.to_string(), f_name))
                } else {
                    None
                }
            })
            .collect()
    ))
}
