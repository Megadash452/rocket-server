use rocket::http::Status;
use rocket::tokio::fs::{self, File};
use rocket_dyn_templates::{Template, context};
use serde_json::json;
use std::path::PathBuf;
use std::io;

static EXCLUDED_DIRS: &[&str] = &[
    "target", ".secrets"
];

/// Allows the user to browse the server's filesystem.
#[get("/<path..>", rank=1)]
pub async fn dir_browser(path: PathBuf) -> ResResult {
    if crate::helpers::eq_one_of(path.to_string_lossy().as_ref(), EXCLUDED_DIRS) {
        return ResResult::Err(Status::Forbidden)
    }

    match dir_entries(&path).await {
        Ok(entries) => ResResult::Dir({
            // Convert DirEntries to JSON values for the template
            let entries = entries.into_iter()
                .map(|entry| json!({
                    "path": PathBuf::from("/files").join(entry.path()),
                    "name": entry.file_name().to_string_lossy().to_string()
                })).collect::<Vec<_>>();

            Template::render("fs", context! { path: path, entries: entries })
        }),
        Err(_) => match file(&path).await {
            Ok(file) => ResResult::File(file),
            Err(error) => ResResult::io_err(error.kind())
        }
    }
}
pub async fn dir_entries(path: &PathBuf) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries = vec![];
    let mut read_dir = fs::read_dir(PathBuf::from(".").join(path)).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue
        }
        entries.push(entry)
    }

    Ok(entries)
}
pub async fn file(path: &PathBuf) -> io::Result<File> {
    File::open(path).await
}

#[derive(Responder)]
pub enum ResResult {
    File(File),
    Dir(Template),
    Err(Status)
}
impl ResResult {
    pub fn io_err(error: io::ErrorKind) -> Self {
        use std::io::ErrorKind;

        Self::Err(match error {
            ErrorKind::NotFound => Status::NotFound,
            ErrorKind::PermissionDenied => Status::Forbidden,
            _ => Status::InternalServerError
        })
    }
}
