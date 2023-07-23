pub mod authenticate;
pub mod osts;
pub mod games;

use std::{path::{PathBuf, Path}, process::Command};
use once_cell::sync::Lazy;
use rocket::{
    // tokio,
    futures::{stream, Stream, StreamExt}
};
use yew::{
    ServerRenderer, Html, Properties, BaseComponent, Children,
    function_component, html,
};

static ICONS_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./res/icons/"));
// The default path where pfps are stored
static DEFAULT_PFP_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./res/users-pfp/"));
// The path to the default pfp
static DEFAULT_PFP: Lazy<String> = Lazy::new(|| {
    let data = std::fs::read("res/icons/default-user.svg").expect("No default pfp file");
    match String::from_utf8(data) {
        Ok(svg) => svg,
        Err(_) => r#"<img class="user-pfp" alt="user-pfp"/>"#.to_string()
    }
});


#[derive(Properties, PartialEq)]
struct HeadProps {
    title: String,
    #[prop_or_default]
    children: Children,
}
#[function_component]
fn Head(props: &HeadProps) -> Html {
    html! {
        <head>
            <meta charset="UTF-8"/>
            <meta http-equiv="X-UA-Compatible" content="IE=edge"/>
            <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
            <style>{ ":root{color-scheme: light dark;}" }</style>
            <title>{ &props.title }</title>
            { for props.children.iter() }
        </head>
    }
}


#[derive(Properties, Default, Clone, PartialEq, Eq)]
pub struct UserInfo {
    #[prop_or_default]
    pub username: Option<String>,
    #[prop_or_default]
    pub pfp_path: Option<PathBuf>
}
impl From<String> for UserInfo {
    /// [`Self::username`] is the string, [`Self::pfp_path`] is [`DEFAULT_PFP_PATH`]`/<username>`.
    fn from(username: String) -> Self {
        Self {
            pfp_path: Some(DEFAULT_PFP_PATH.join(&username)),
            username: Some(username)
        }
    }
}
impl From<Option<String>> for UserInfo {
    fn from(username: Option<String>) -> Self {
        match username {
            Some(username) => Self::from(username),
            None => Self::default()
        }
    }
}
impl From<Option<crate::auth::User>> for UserInfo {
    fn from(user: Option<crate::auth::User>) -> Self {
        match user {
            Some(user) => Self {
                username: Some(user.name),
                pfp_path: user.pfp_path
            },
            None => Self::default()
        }
    }
}

#[function_component]
fn PageHeader(props: &UserInfo) -> Html {
    html! {
        <header id="page-header">
            <div class="left">
                <div class="menu-wrapper">
                    <input type="checkbox"
                        role="button"
                        aria-label="main menu" 
                        aria-controls="main-menu"
                        /* aria-expanded="false" */ />
                    <ul id="main-menu">
                        
                    </ul>
                </div>
            </div>
            <div class="right">
                <div id="user-controls">
                    <a role="button" href="/login">
                        <div class="pfp-wrapper">{ find_pfp(&props.pfp_path) }</div>
                        <span>{
                            match &props.username {
                                Some(username) => username.as_str(),
                                None => "Log In"
                            }
                        }</span>
                    </a>
                    // If is logged in
                    if props.username.is_some() {
                        <hr/>
                        <div class="menu-wrapper">
                            <input type="checkbox"
                                role="button"
                                aria-label="user-controls menu" 
                                aria-controls="user-controls-menu"
                                /* aria-expanded="false" */ />
                            <div class="bottom-align">
                                <ul id="user-controls-menu">
                                    <li><a href="/logout">{ "Log out" }</a></li>
                                </ul>
                            </div>
                        </div>
                    }
                </div>
            </div>
        </header>
    }
}


/// Helper that tries to find the image of a user's profile-picture.
/// Returns an `<img>` element linking to the file.
/// 
/// If an image is not found in the specified **path**,
/// an `SVG` element of the default pfp is returned instead.
/// 
/// This function requires the `ImageMagick` package.
fn find_pfp(path: &Option<PathBuf>) -> Html {
    if let Some(path) = path {
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.is_file()
            && Command::new("identify")
                .arg(path)
                .output()
                .is_ok() // file is an image
            {
                return html!(<img src={PathBuf::from("/files/").join(path).display().to_string()} alt="PFP"/>)
            }
        }
    }
    Html::from_html_unchecked(yew::AttrValue::from(DEFAULT_PFP.clone()))
}

fn text_file(path: &Path) -> Html {
    let content = String::from_utf8_lossy(
        &std::fs::read(path).expect("Could not read file")
    ).to_string();

    if path.extension().is_some_and(|ext| ext == "md") {
        // Markdown files can be compiled to HTML.
        Html::from_html_unchecked(markdown::to_html(&content).into())
    } else {
        html! { <code>{ content }</code> }
    }
}

fn item_error(obj_name: String, error: String) -> Html {
    html!{
        <li class="error horizontal-wrapper">
            <Icon name="warning"/>
            <div class="vertical-wrapper">
                <span class="name">{obj_name}</span>
                <span class="error">{error}</span>
            </div>
        </li>
    }
}

// pub fn load_svg(name: impl AsRef<str>) -> Html {
//     // TODO: use cache
//     let data = std::fs::read(ICONS_PATH.join(name.as_ref()).with_extension("svg")).ok();

//     Html::from_html_unchecked(yew::AttrValue::from(match data {
//         Some(svg) => crate::helpers::command_output(svg),
//         None => r#"<img alt="SVG"/>"#.to_string()
//     }))
// }
#[derive(Properties, PartialEq, Eq)]
pub struct IconProps {
    name: yew::AttrValue
}
#[function_component]
pub fn Icon(props: &IconProps) -> Html {
    // TODO: use cache
    let data = std::fs::read(ICONS_PATH.join(props.name.as_str()).with_extension("svg")).ok();

    Html::from_html_unchecked(yew::AttrValue::from(match data {
        Some(svg) => crate::helpers::command_output(svg),
        None => r#"<img alt="SVG"/>"#.to_string()
    }))
}


#[derive(Properties, Default, PartialEq)]
pub struct DocProps {
    pub title: String,
    pub header: UserInfo,
    #[prop_or_default]
    pub children: Children
}
/// A helper struct for other components. Renders a page with the correct header and style.
#[function_component]
pub fn Document(props: &DocProps) -> Html {
    html! {
        <html lang="en">
            <Head title={ props.title.clone() }>
                <link rel="stylesheet" type="text/css" href="/style.css"/>
            </Head>
            <body>
                <PageHeader ..props.header.clone()/>
                <main>
                    { for props.children.iter() }
                </main>
            </body>
        </html>
    }
}


#[derive(Properties, Default, PartialEq, Eq)]
pub struct InnerHtmlProps {
    pub title: String,
    pub header: UserInfo,
    pub content: String
}
/// Same as [`Document`] but the [`Chilren`] is an html string (useful for html compiled from markdown)
#[function_component]
pub fn InnerHtml(props: &InnerHtmlProps) -> Html {
    html! {
        <html lang="en">
            <Head title={ props.title.clone() }>
                <link rel="stylesheet" type="text/css" href="/style.css"/>
            </Head>
            <body>
                <PageHeader ..props.header.clone()/>
                <main>
                    { Html::from_html_unchecked(props.content.clone().into()) }
                </main>
            </body>
        </html>
    }
}


// #[tokio::main]
// pub async fn static_render<Comp: BaseComponent>(props: Comp::Properties) -> String
// where Comp::Properties: Send
// {
//     "<!DOCTYPE html>".to_string() + ServerRenderer::<Comp>::with_props(|| props).render().await.as_str()
// }

/// ### Note:
/// When is returned in async function with `TextStream![String]`,
/// must be wrapped in some other type (such as [`RawHtml`](rocket::response::content::RawHtml)).
pub fn render<Comp: BaseComponent>(props: Comp::Properties) -> impl Stream<Item = String> + Send
where Comp::Properties: Send
{
    stream::once(async { "<!DOCTYPE html>".to_string() })
        .chain(ServerRenderer::<Comp>::with_props(|| props).render_stream())
}

// #[inline]
// pub fn render_default<Comp: BaseComponent>() -> impl Stream<Item = String> + Send
// where Comp::Properties: Send + Default
// {
//     render::<Comp>(Comp::Properties::default())
// }
