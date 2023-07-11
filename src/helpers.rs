use std::{
    io,
    path::{PathBuf, Path},
    fmt::{Display, Write},
    fs::{Metadata, DirEntry},
    process::{Command, Stdio}
};
use chrono::Duration;
use serde::Deserialize;


#[macro_export]
macro_rules! do_while {
    (do $body:block while $cond:expr) => {
        while { $body; $cond } {}
    };
}


#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Dep {
    File {
        name: String,
        download: String,
        instructions: String
    },
    Package {
        name: String,
        // TODO: May add install command for other distros
        install: String
    }
}
impl Dep {
    pub fn name(&self) -> &str {
        match self {
            Self::Package { name, .. } => name,
            Self::File { name, .. } => name
        }
    }
    pub fn installation(&self) -> String {
        match self {
            Self::Package { install, .. } => format!("Run command `{install}`"),
            Self::File { download, instructions, .. } => format!("Download from {download:?}, then {instructions}")
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ExternalDeps {
    commands: Vec<Dep>,
    python: Vec<Dep>,
}
impl ExternalDeps {
    /// Returns `false` if there are Missing Dependencies.
    pub fn resolve(self) -> bool {
        let mut good = true;

        for dep in self.commands {
            let exists = Command::new("command")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .arg("-v")
                .arg(dep.name())
                .status()
                .is_ok_and(|status| status.success());
            if !exists {
                eprintln!("Missing {:?} command. Installation:\n\t{}", dep.name(), dep.installation());
            }
            // If was good, is not good now
            if good {
                good = exists
            }
        }

        for dep in self.python {
            let exists = Command::new("python")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .arg("-c")
                .arg(format!("import {}", dep.name()))
                .status()
                .is_ok_and(|status| status.success());
            if !exists {
                eprintln!("Missing python module {:?}. Installation:\n\t{}", dep.name(), dep.installation())
            }
            // If was good, is not good now
            if good {
                good = exists
            }
        }

        good
    }
}


/// Removes the text up to (and including) the first point `.` ecountered from the back.
/// Does nothing if there is no point `.` in the string.
/// 
/// e.g. `"index.html" -> "index"`, `"file" -> "file"`.
pub fn strip_extension(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().with_extension("")
}

pub fn eq_one_of<'a>(this: &'a str, others: impl AsRef<[&'a str]>) -> bool {
    for &other in others.as_ref() {
        if this == other {
            return true
        }
    }

    false
}

pub fn resolve_symlink(mut link: PathBuf) -> io::Result<PathBuf> {
    while link.symlink_metadata()?.is_symlink() {
        link = std::fs::read_link(link)?
    }
    Ok(link)
}

pub fn resolve_entry(entry: DirEntry) -> Option<(PathBuf, Metadata)> {
    let mut path = entry.path();
    let mut meta = entry.metadata().ok()?;
    
    if meta.is_symlink() {
        path = crate::helpers::resolve_symlink(path).ok()?;
        meta = path.metadata().ok()?;
    }
    Some((path, meta))
}

/// Removes the trailing `'\n'` from a [`Command`]'s output (stdout or stderr).
pub fn command_output(mut output: Vec<u8>) -> String {
    if output.last().is_some_and(|&l| l == b'\n') {
        output.pop();
    }
    String::from_utf8_lossy(&output).to_string()
}

/// Find files that start with some string.
pub fn find_files_start(search_dir: impl AsRef<Path>, start: &str, case_sensitive: bool) -> Vec<PathBuf> {
    Command::new("find")
        .arg("-L")
        .arg(search_dir.as_ref())
        .arg("-maxdepth")
        .arg("1")
        .arg(if case_sensitive {
            "-name"
        } else {
            "-iname"
        })
        .arg(format!("{start}*"))
        .output()
        .map(|o|
            command_output(o.stdout)
                .split('\n')
                .filter(|s| !s.is_empty())
                .map(|s| PathBuf::from(s))
                .collect::<Vec<_>>()
        )
        .unwrap_or_default()
}

pub fn display_separated<T: Display>(things: impl AsRef<[T]>, separator: &str) -> String {
    things.as_ref().iter()
        .map(T::to_string)
        .intersperse(separator.to_string())
        .collect()
}

pub fn display_duration(secs: u64) -> String {
    let duration = Duration::seconds(secs as i64);
    let mut buf = String::new();

    if duration.num_hours() > 0 {
        write!(buf, "{}:", duration.num_hours()).unwrap()
    }
    write!(buf, "{:02}:{:02}", duration.num_minutes() % 60, duration.num_seconds() % 60).unwrap();

    buf
}


/// Make [`PathBuf`] usable as rocket mounting point.
/// ```no_run
/// .mount(path.rocket_base(), routes![])
/// ```
pub trait PathExt {
    fn rocket_base(&self) -> String;
}
impl PathExt for PathBuf {
    fn rocket_base(&self) -> String {
        PathBuf::from("/").join(self).to_string_lossy().to_string()
    }
}
