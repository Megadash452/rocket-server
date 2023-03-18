use argon2::{
    Argon2, PasswordHasher,
    password_hash::{PasswordHashString, SaltString, errors::Error as HashError}
};
use rand_core::OsRng;
use crate::do_while;
use super::db::{Users, UserNameError};


pub fn create_pass_hash(password: &[u8]) -> Result<PasswordHashString, HashError> {
    let salt = {
        let mut salt;
        // regenerate `salt` while it contains ':'
        do_while!{ do {
            salt = SaltString::generate(&mut OsRng);
        } while salt.as_str().contains(Users::SEP) }

        salt
    };
    // let pepper = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default().hash_password(password, &salt)?;

    Ok(password_hash.serialize())
}

/// If invalid, returns the [`char`] that makes this username invalid.
pub fn validate_username(username: &str) -> Result<(), UserNameError> {
    if username.is_empty() {
        return Err(UserNameError::Empty)
    }
    for ch in username.chars() {
        if !validate_username_char(ch) {
            return Err(ch.into())
        }
    }
    
    Ok(())
}
/// Returns `true` for any valid [`char`].
/// Returns `false` if **ch** is not in any of the ranges in [`User::ALLOWED_NAME_CHARS`].
pub fn validate_username_char(ch: char) -> bool {
    for range in Users::ALLOWED_NAME_CHARS {
        // If the character is not valid for an UserName, throw error
        if range.contains(&(ch as u8)) {
            return true
        }
    }

    false
}

pub async fn admin_user_exists(users: &Users) -> bool {
    users.usernames().await
        .find(|user_id|
            user_id == super::db::ADMIN_USR_ID)
        .is_some()
}


#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::fs;
    use std::{path::PathBuf, error::Error};
    use rocket::tokio;
    use super::Users;

    
    fn temp() -> Result<PathBuf, Box<dyn Error>> {
        Ok(PathBuf::from({
            let mut stdout = Command::new("mktemp")
                .output()?
                .stdout;
            stdout.pop();
            String::from_utf8(stdout)?
        }))
    }

    #[test]
    fn load() {
        let path = temp().unwrap();
        fs::write(&path,
            b"$admin$113$argon2id$v=19$m=4096,t=3,p=1$DkiuneDgPzT0wJDiNly1TQ$TBPaDhAzZNnvSEQVHHy5yd/Ih34jwHkRJDTP9Yy+KG5gpLvfC/siR9NFJ9GK\n$viewer$113$argon2id$v=19$m=4096,t=3,p=1$M92l6PXdp3JfUtLd5mfD2Q$DxDiy3w54NcvTvxrzT4JbM8zreWimW8HMCit1vZ+Yczes9u8Yu0pVBGoRCxt"
        ).unwrap();
        println!("file:\n{}", String::from_utf8(std::fs::read(&path).unwrap()).unwrap());

        let db = Users::load_path(path.clone()).unwrap();
        dbg!(db);
    }

    #[tokio::test]
    async fn add_user() {
        let path = temp().unwrap();
        let db = Users::load_path(path.clone()).unwrap();

        db.add_user("admin", "password").await.unwrap();
        db.add_user("viewer", "password").await.unwrap();
        println!("file:\n{}", String::from_utf8(std::fs::read(path).unwrap()).unwrap());
    }
}
