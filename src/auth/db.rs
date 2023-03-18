use std::{
    io,
    collections::HashMap,
    path::PathBuf,
    ops::RangeInclusive,
    num::ParseIntError
};
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHashString, errors::Error as HashError}
};
use rand::{distributions::Alphanumeric, Rng};
use rocket::tokio::{/*self,*/ fs, io::AsyncWriteExt};
use once_cell::sync::Lazy;
use async_std::sync::Mutex as AsyncMutex;
use thiserror::Error;
use crate::do_while;
use super::helpers;

type Cookie = rocket::http::Cookie<'static>;
static USERS_FILE: Lazy<PathBuf> = Lazy::new(|| PathBuf::from(".secrets/db/users"));
static NON_ASCII_PASS_MSG: &str = "Password must only contain ASCII characters";
pub static ADMIN_USR_ID: &str = "admin";


#[derive(Debug)]
pub struct Users {
    path: PathBuf,
    // TODO: maybe use RwLock
    db: AsyncMutex<HashMap<String, PasswordHashString>>,
    /// `HashMap<SessionUuid, UserId>`
    /// 
    /// When server shuts down, all sessions are deleted, and all users are logged out.
    sessions: AsyncMutex<HashMap<String, String>>
}
impl Users {
    /// The character that separates [`User`] components (e.g. name, salt, ...).
    pub const SEP: char = '$';
    /// Separates each entry in the file. E.g. `$user0$...\n$user1`
    const ENTRY_SEP: char = '\n';
    /// Ranges of the allowed ASCII chars in the `UserName`.
    pub const ALLOWED_NAME_CHARS: [RangeInclusive<u8>; 4] = [
        48..=57, // Integers 0-9
        64..=90, // @ and uppercase A-Z
        95..=95, // underscore _
        97..=122, // lowercase a-z
    ];

    /// Load existing [`Users`] from a file.
    // #[tokio::main]
    pub fn load_path(path: PathBuf) -> Result<Self, LoadUsersError> {
        use std::fs;
        use std::process::Command;

        let file = match fs::read_to_string(&path) {
            Ok(file) => file,
            // If file does not exist, create it
            Err(_) => {
                Command::new("mkdir")
                    .arg("-p")
                    .arg(match path.parent() {
                        Some(path) => path, // path is a file within a directory
                        None => &path // path is a file in the server's root
                    }).status()?;
                
                Command::new("touch")
                    .arg(&path)
                    .status()?;
                
                fs::read_to_string(&path)?
            }
        };
        
        Ok(Self {
            db: AsyncMutex::new(Self::db_from_str(&file)?),
            sessions: AsyncMutex::new(HashMap::new()),
            path
        })
    }
    #[inline]
    pub fn load_default() -> Result<Self, LoadUsersError> {
        Self::load_path(USERS_FILE.clone())
    }

    /// Used for registering new users.
    /// Checks that **username** and **password** are both valid.
    pub async fn add_user(&self, username: &str, password: &str) -> Result<Cookie, RegisterError> {
        let db = &mut *self.db.lock().await;

        if db.contains_key(username) {
            return Err(RegisterError::ExistingUser)
        }

        let password = if password.is_ascii() {
            password.as_bytes()
        } else {
            return Err(RegisterError::NonAsciiPassword)
        };

        helpers::validate_username(username)?;
        let hash = helpers::create_pass_hash(password)?;
        
        // Write new User to file before putting it in the server state, as this can fail.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.path).await?;

        file.write_all(unsafe {
            format!("{sep}{username}{sep}{}{}",
                hash.as_str().chars().count(), hash, sep=Self::SEP
            ).as_bytes_mut()
        }).await?;
        file.write_u8(b'\n').await?;
        
        db.insert(username.to_string(), hash.clone());

        Ok(self.new_session(username).await)
    }

    /// Used for loging in existing users.
    pub async fn verify_user(&self, username: &str, password: &str) -> Result<Cookie, LoginError> {
        let password = if password.is_ascii() {
            password.as_bytes()
        } else {
            return Err(LoginError::NonAsciiPassword)
        };

        let hash = match self.db.lock().await.get(username) {
            Some(hash) => hash.clone(),
            None => return Err(LoginError::UnknownUser)
        };
        let hash = hash.password_hash();

        match Argon2::default().verify_password(password, &hash) {
            Ok(()) => Ok(self.new_session(username).await),
            Err(_) => Err(LoginError::WrongPassword)
        }
    }

    /// If is a valid session, returns the `user id` of that session
    pub async fn validate_session(&self, session_uuid: &str) -> Option<String> {
        self.sessions.lock().await
            .get(session_uuid)
            .and_then(|user_id| Some(user_id.clone()))
    }

    pub async fn remove_session(&self, session_uuid: &str) {
        self.sessions.lock().await.remove(session_uuid);
    }

    async fn new_session(&self, username: &str) -> Cookie {
        let sessions = &mut *self.sessions.lock().await;

        let mut uuid: String;
        // Ensure the uuid is unique
        do_while!{ do {
            uuid = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(37)
                .map(char::from)
                .collect();
        } while sessions.contains_key(&uuid) };

        // Save session
        sessions.insert(uuid.clone(), username.to_string());

        Cookie::build(super::SESSION_COOKIE, uuid)
            .secure(true)
            .http_only(true)
            .same_site(rocket::http::SameSite::Strict)
            .finish()
            .into_owned()
    }

    /// Returns an [`Iterator`] over all the `users ids` registered in the server.
    pub(super) async fn usernames(&self) -> impl Iterator<Item = String> {
        self.db.lock().await
            .keys()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Format: `$username$PasswordHashLength$PasswordHash`.
    /// Each user separated by a *line-break* `\n`.
    fn db_from_str(s: &str) -> Result<HashMap<String, PasswordHashString>, LoadUsersError> {
        #[derive(PartialEq)]
        enum State {
            Name, HashLen
        }
        #[derive(Default)]
        struct Builder {
            name: String,
        }

        let mut db = HashMap::new();
        let mut builder = Builder::default();
        let mut buf = String::new();
        let mut chars = s.chars();
        let mut state = None;

        while let Some(ch) = chars.next() {
            if ch == Self::SEP {
                match state {
                    // Every entry starts with `Self::SEP`
                    None => state = Some(State::Name),

                    Some(State::Name) => {
                        if buf.is_empty() {
                            return Err(LoadUsersError::InvalidUserName(UserNameError::Empty))
                        }
                        builder.name = buf;
                        buf = String::new();
                        state = Some(State::HashLen)
                    }
                    Some(State::HashLen) => {
                        // The length of the PasswordHashString
                        let len = buf.parse::<usize>()
                            .map_err(|error| LoadUsersError::BadHashLength(error))?;
                        buf = String::with_capacity(len);

                        buf.push(Self::SEP);
                        let mut count = 1; // already read the '$'
                        // Read `len` characters of PasswordHash 
                        while let Some(ch) = chars.next() {
                            buf.push(ch);
                            count += 1;
                            if count == len {
                                break
                            }
                        }
                        if count != len {
                            // File ended before enough hash characters could be read
                            return Err(LoadUsersError::IncompleteHash)
                        }

                        // After each entry there should be a line to separate Users
                        match chars.next() {
                            Some(Self::ENTRY_SEP) | None => {},
                            Some(_) => return Err(LoadUsersError::InvalidEntrySep(ch))
                        }

                        db.insert(builder.name, PasswordHash::new(&buf)?.serialize());
                        builder = Builder::default();
                        buf = String::new();
                        state = None
                    }
                }

                continue
            }
            match state {
                // When entry does not start with `Self::SEP`
                None => return Err(LoadUsersError::InvalidEntry),
                // If reading UserName, make sure it has valid chars
                Some(State::Name) =>
                    if !helpers::validate_username_char(ch) {
                        return Err(LoadUsersError::InvalidUserName(ch.into()))
                    }
                _ => {}
            }

            buf.push(ch);
        }

        Ok(db)
    }
}

#[derive(Error, Debug)]
pub enum LoadUsersError {
    #[error("Could not read the length of the PasswordHash data")]
    BadHashLength(#[from] ParseIntError),
    #[error("Length of PasswordHash does not match its actual length")]
    IncompleteHash,
    #[error("Each user entry in the file must be separated by a {:?}", Users::ENTRY_SEP)]
    InvalidEntrySep(char),
    #[error("Each entry must start with {:?}", Users::SEP)]
    InvalidEntry,
    #[error("{0}")]
    InvalidUserName(#[from] UserNameError),
    #[error("Error hashing password: {0:}")]
    InvalidHash(HashError),
    #[error("Error reading \"users\" database file")]
    IoError(#[from] io::Error)
}
impl From<HashError> for LoadUsersError {
    #[inline]
    fn from(value: HashError) -> Self {
        Self::InvalidHash(value)
    }
}

#[derive(Error, Debug)]
pub enum RegisterError {
    #[error("Username already exists")]
    ExistingUser,
    #[error("{NON_ASCII_PASS_MSG}")]
    NonAsciiPassword,
    #[error("{0}")]
    InvalidName(#[from] UserNameError),
    #[error("Error hashing password: {0}")]
    HashError(HashError),
    /// Error when appending to users file.
    #[error("Error saving password hash: {0:?}")]
    IoError(#[from] io::Error)
}
impl From<HashError> for RegisterError {
    fn from(value: HashError) -> Self {
        Self::HashError(value)
    }
}

#[derive(Error, Debug)]
pub enum LoginError {
    #[error("{NON_ASCII_PASS_MSG}")]
    NonAsciiPassword,
    #[error("Username not found")]
    UnknownUser,
    #[error("Wrong password")]
    WrongPassword
}

#[derive(Error, Debug)]
pub enum UserNameError {
    #[error("Username must not be empty")]
    Empty,
    /// Found a [`char`] that can't be in a UserName.
    #[error("Username cannot contain character {0:?}")]
    Char(char)
}
impl From<char> for UserNameError {
    #[inline]
    fn from(ch: char) -> Self {
        Self::Char(ch)
    }
}
impl From<Option<char>> for UserNameError {
    fn from(value: Option<char>) -> Self {
        match value {
            Some(ch) => Self::Char(ch),
            None => Self::Empty
        }
    }
}
