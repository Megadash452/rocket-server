use async_process::Command;
use rocket::response::content::RawCss as Css;
use super::*;

pub static CSS_ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("target/css"));


#[get("/<path..>", format="text/css")]
/// Compile SASS stylesheets in [`ROUTES_ROOT`] and serve them as css.
/// Caches the compiled css as a file for later use.
pub async fn serve_css(path: PathBuf) -> io::Result<Css<File>> {
    let path = ROUTES_ROOT.join(path);
    let dir = {
        let mut path = path.clone();
        path.pop();
        path
    };
    let no_ext = strip_extension(&path);
    let expected_css = CSS_ROOT.join(&no_ext).with_extension("css");

    Command::new("sass")
        .arg("--update")
        .arg("--no-source-map")
        .arg("--style=compressed")
        .arg(format!("--load-path={dir:?}"))
        .arg(no_ext.with_extension("sass")) // input
        .arg(&expected_css) // output
        .output().await?;

    Ok(Css(File::open(expected_css).await?))
}
