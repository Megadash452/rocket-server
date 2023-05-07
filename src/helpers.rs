use std::{path::{PathBuf, Path}, fmt::{Display, Write}, process::Command};
use chrono::Duration;


#[macro_export]
macro_rules! do_while {
    (do $body:block while $cond:expr) => {
        while { $body; $cond } {}
    };
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
