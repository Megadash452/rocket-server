use async_process::Command;
use rocket::{
    response::content::RawCss as Css,
    request::FromSegments,
    http::uri::{Segments, fmt::Path as RocketPath, error::PathError as RocketPathError},
};
use super::*;

pub static CSS_ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("target/css"));


/// A path that a client would request a CSS file.
/// The SASS file found in that path is compiled to CSS and served.
/// 
/// Path is the actual file path (without .sass extension) of the intended stylesheet.
/// E.g. requested `/dir/style.css`, is `SassPath("/routes/dir/style")`
#[derive(Debug)]
pub struct SassPath(PathBuf);
impl FromSegments<'_> for SassPath {
    type Error = SassPathError;

    fn from_segments(segments: Segments<'_, RocketPath>) -> Result<Self, Self::Error> {
        let path = ROUTES_ROOT.join(
            PathBuf::from_segments(segments)
                .map_err(|e| SassPathError::PathError(e))?
        );

        if !path.extension().is_some_and(|ext| ext == "css") {
            return Err(SassPathError::NotCss)
        }
        if !path.with_extension("sass").is_file() {
            return Err(SassPathError::NoSass)
        }
        Ok(Self(strip_extension(path)))
    }
}

#[derive(Debug)]
pub enum SassPathError {
    PathError(RocketPathError),
    /// Client did not ask for CSS.
    NotCss,
    /// No .sass file exists.
    NoSass,
}

#[get("/<path..>", rank=0)]
/// Compile SASS stylesheets in [`ROUTES_ROOT`] and serve them as css.
/// Caches the compiled css as a file for later use.
/// 
/// Forwards request if no such file (sass) exists.
pub async fn serve_css(path: SassPath) -> io::Result<Css<File>> {
    let path = path.0;
    let dir = {
        let mut path = path.clone();
        path.pop();
        path
    };
    let expected_css = CSS_ROOT.join(&path).with_extension("css");

    Command::new("sass")
        .arg("--update")
        .arg("--no-source-map")
        .arg("--style=compressed")
        .arg(format!("--load-path={dir:?}"))
        .arg(path.with_extension("sass")) // input
        .arg(&expected_css) // output
        .output().await?;

    Ok(Css(File::open(expected_css).await?))
}
