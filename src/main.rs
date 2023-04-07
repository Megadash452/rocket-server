#![feature(is_some_and)]
#![feature(iter_intersperse)]

mod file_browser;
mod helpers;
mod sass;
mod archives;
mod auth;
mod components;
#[cfg(test)] mod tests;

use std::{
    io,
    path::PathBuf,
    process::Command,
    collections::HashMap,
};
use rocket::{
    Request, State, Config,
    response::{
        Redirect,
        content::RawHtml,
        stream::TextStream
    },
    http::Status,
    tokio::fs::File,
    form::Form,
    figment::Figment,
};
use rocket_dyn_templates::Template;
use once_cell::sync::Lazy;
use components::render as render_component;
use helpers::*;
#[macro_use] extern crate rocket;

type Icons = HashMap<PathBuf, String>;

static ROUTES_ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./routes/"));
/// Where yew components are rendered to
// static RENDER_ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("target/static-html"));
static SECRET_KEY_PATH: &str = "./.secrets/secret-key";


// #[get("/")]
// /// Serve the index.html._ at each directory.
// async fn root_index(icons: &State<Icons>) -> Template {
//     Template::render("index", context! { icons: &**icons })
// }
#[get("/")]
fn index_md(user: Option<auth::User>) -> RawHtml<TextStream<impl async_std::stream::Stream<Item=String>>> {
    RawHtml(TextStream(components::render::<components::InnerHtml>(
        components::InnerHtmlProps {
            title: "Home".to_string(),
            header: user.into(),
            // TODO: cache markdown output
            content: markdown::to_html(&std::fs::read_to_string("./routes/index.md").expect("no index file"))
        }
    )))
}
// #[get("/")]
// async fn index_md(user: Option<auth::User>) -> RawHtml<String> {
//     RawHtml(components::static_render::<components::Document>(
//         components::DocProps {
//             header: user.into(),
//             content: markdown::to_html(&std::fs::read_to_string("./routes/index.md").expect("no index file"))
//         }
//     ))
// }

#[get("/favicon.ico")]
async fn favicon() -> io::Result<File> {
    File::open("./res/icons/favicon.ico").await
}
// TODO: have one that detects file extension and forards it to the right handler (e.g. "index.html" -> index_template, "file" -> regular file)

// #[catch(default)]
// fn default_catcher(status: Status, _request: &Request) -> Template {
//     Template::render("error", context! { code: status.code, reason: status.reason_lossy() })
// }


fn rocket_config() -> Figment {
    Config::figment()
        .merge(("template_dir", "./"))
        .merge(("port", 8000))
        .merge(("secret_key", match std::fs::read(SECRET_KEY_PATH) {
            Ok(key) => key,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied =>
                panic!("Give file \"{SECRET_KEY_PATH}\" permissions 640"),

            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Command::new("touch")
                    .arg(SECRET_KEY_PATH).output()
                    .expect(&format!("Can't create file \"{SECRET_KEY_PATH}\" to store secret key"));
                Command::new("chmod")
                    .arg("640").output()
                    .expect(&format!("Can't modify permissions for \"{SECRET_KEY_PATH}\""));
                // openssl rand -base64 32 > {SECRET_KEY_PATH}
                std::fs::write(SECRET_KEY_PATH, Command::new("openssl")
                    .arg("rand")
                    .arg("-base64")
                    .arg("32")
                    .output().expect("Can't create secret key")
                    .stdout).unwrap();

                std::fs::read(SECRET_KEY_PATH).unwrap()
            },

            Err(error) => panic!("{error}")
        }))
}

#[launch]
fn rocket_server() -> _ {
    let rocket = rocket::custom(rocket_config())
        // .mount(projects::ROOT.rocket_base(), projects::routes())
        // .mount(projects::ROOT.rocket_base(), FileServer::from("local-replit"))
        // Auth
        .mount("/login", auth::login::routes())
        .mount("/logout", auth::logout::routes())
        .mount("/register", auth::register::routes())
        .mount("/admin-register", auth::admin_register::routes())
        // Base
        .mount("/", routes![sass::serve_css])
        .mount("/", routes![index_md, favicon])
        .mount("/files", routes![file_browser::dir_browser])
        // Archives
        .mount("/osts", archives::osts::routes())
        .mount("/games", archives::games::routes())
        
        .attach(Template::fairing())
        .manage(auth::db::Users::load_default().unwrap()) // load db/users
        .manage(std::fs::read_dir("./res/icons").unwrap() // icons
            .filter_map(|entry| {
                let entry = entry.ok()?;
                entry.metadata().ok()?.is_file().then_some(entry)
            })
            .filter_map(|entry|
                Some(strip_extension(entry.file_name()))
                    .zip(std::fs::read_to_string(entry.path()).ok())
            ).collect::<Icons>()
        );

    // TODO:
    for dir in std::fs::read_dir(&*ROUTES_ROOT)
        .expect("must have routes dir")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.metadata().unwrap().is_dir())
    {
        println!("dir: {dir:?}");
    }

    rocket  
}
