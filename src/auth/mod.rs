pub mod db;
mod helpers;

use rocket::{
    Route,
    request::{FromRequest, FlashMessage, Outcome},
    response::{
        Flash,
        content::RawHtml as Html
    },
    http::{Cookie, CookieJar},
    outcome::IntoOutcome
};
use super::*;

pub static SESSION_COOKIE: &str = "session_uuid";


/// Request Guard requiring the the request to come from an admin session
struct Admin;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Admin {
    type Error = ();

    /// Returns error when can't access the server's sessions
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session = match req.cookies().get_private(SESSION_COOKIE) {
            Some(session) => session,
            None => return Outcome::Forward(())
        };
        let users = match req.rocket().state::<db::Users>() {
            Some(users) => users,
            None => return Outcome::Failure((Status::InternalServerError, ()))
        };

        // See if the user id for this session is the admin
        users.validate_session(session.value()).await
            .and_then(|user|
                (user == db::ADMIN_USR_ID).then_some(Self))
            .or_forward(())
    }
}

/// Info about an user's session.
pub struct User {
    pub name: String,
    pub pfp_path: Option<PathBuf>,
    // pub group: String,
}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session = match req.cookies().get_private(SESSION_COOKIE) {
            Some(session) => session,
            None => return Outcome::Forward(())
        };
        let users = match req.rocket().state::<db::Users>() {
            Some(users) => users,
            None => return Outcome::Failure((Status::InternalServerError, ()))
        };

        match users.validate_session(session.value()).await {
            Some(username) =>
                Outcome::Success(Self {
                    name: username,
                    pfp_path: None // TODO: get pfp path from users-info file
                }),
            None => return Outcome::Forward(())
        }
    }
}


#[derive(Debug, FromForm)]
struct Creds<'a> {
    username: &'a str,
    password: &'a str
}


pub mod login {
    use rocket::response::Flash;
    use super::*;


    // #[get("/")]
    // async fn index(jar: &CookieJar<'_>, users: &State<db::Users>) -> Result<io::Result<Html<File>>, Redirect> {
    //     // If no admin user exists, redirect to create one
    //     if !admin_user_exists(users).await {
    //         return Err(Redirect::to("/admin-register"))
    //     }
    //
    //     // If user is trying to log in but is already logged in, redirect to home page
    //     if let Some(cookie) = jar.get_private(SESSION_COOKIE) {
    //         if let Some(_) = users.validate_session(cookie.value()).await {
    //             return Err(Redirect::to("/"))
    //         }
    //         // Remove user's session_uuid if it is invalid
    //         jar.remove_private(Cookie::named(SESSION_COOKIE));
    //     }
    //
    //     Ok(File::open(RENDER_ROOT.join("login.html")).await
    //         .and_then(|file| Ok(Html(file)))
    //     )
    // }
    #[get("/")]
    async fn index(jar: &CookieJar<'_>, users: &State<db::Users>, error: Option<FlashMessage<'_>>) -> Result<Html<TextStream![String]>, Redirect> {
        // If no admin user exists, redirect to create one
        if !helpers::admin_user_exists(users).await {
            return Err(Redirect::to("/admin-register"))
        }

        // If user is trying to log in but is already logged in, redirect to home page
        if let Some(cookie) = jar.get_private(SESSION_COOKIE) {
            if let Some(_) = users.validate_session(cookie.value()).await {
                return Err(Redirect::to("/"))
            }
            // Remove user's session_uuid if it is invalid
            jar.remove_private(Cookie::named(SESSION_COOKIE));
        }

        Ok(Html(TextStream(crate::components::render::<crate::components::authenticate::Login>(error.into()))))
    }

    #[post("/", data="<creds>")]
    async fn login(jar: &CookieJar<'_>, users: &State<db::Users>, creds: Form<Creds<'_>>) -> Result<Redirect, Flash<Redirect>> {
        let cookie = users.verify_user(creds.username, creds.password).await
            .map_err(|error| Flash::error(Redirect::to("/login"), error.to_string()))?;
        jar.add_private(cookie);
        // TODO: read the "prev_page" flash cookie and redirect there
        Ok(Redirect::to("/"))
    }

    pub fn routes() -> Vec<Route> {
        routes![index, login]
    }
}


pub mod logout {
    use super::*;

    #[get("/")]
    async fn logout(jar: &CookieJar<'_>, users: &State<db::Users>) -> Redirect {
        if let Some(cookie) = jar.get_private(SESSION_COOKIE) {
            users.remove_session(cookie.value()).await;
            jar.remove_private(Cookie::named(SESSION_COOKIE));
        }
        Redirect::to("/")
    }

    pub fn routes() -> Vec<Route> {
        routes![logout]
    }
}


/// Only admin can register new users.
pub mod register {
    use rocket::response::{Flash, status::Forbidden};
    use super::*;

    #[get("/", rank = 2)]
    async fn index(users: &State<db::Users>) -> Result<Forbidden<&'static str>, Redirect> {
        // If no admin user exists, redirect to create one
        if !helpers::admin_user_exists(users).await {
            Err(Redirect::to("/admin-register"))
        } else {
            Ok(Forbidden(Some("Please contact admin to create an account")))
        }
    }

    // #[get("/", rank = 1)]
    // async fn index_admin(_admin: Admin) -> io::Result<Html<File>> {
    //     Ok(Html(File::open(RENDER_ROOT.join("register.html")).await?))
    // }
    #[get("/", rank = 1)]
    async fn index_admin(_admin: Admin, error: Option<FlashMessage<'_>>) -> Html<TextStream![String]> {
        Html(TextStream(crate::components::render::<crate::components::authenticate::Register>(error.into())))
    }

    #[post("/", data="<creds>")]
    async fn register(
        jar: &CookieJar<'_>,
        users: &State<db::Users>,
        _admin: Admin,
        creds: Form<Creds<'_>>
    ) -> Result<Redirect, Flash<Redirect>> {
        let cookie = users.add_user(creds.username, creds.password).await
            .map_err(|error| Flash::error(Redirect::to("/register"), error.to_string()))?;
        jar.add_private(cookie);
        // TODO: read the "prev_page" cookie and redirect there
        Ok(Redirect::to("/"))
    }

    pub fn routes() -> Vec<Route> {
        routes![index, index_admin, register]
    }
}

/// Forwarded from [`login`] and [`register`] when no `admin` user is found.
/// Can only be accessed if the is no `admin` user.
pub mod admin_register {
    use rocket::response::status::Forbidden;
    use super::*;

    #[derive(Responder)]
    enum Outcome<S, E> {
        Ok(S),
        #[response(status = 403)]
        Forbidden(()),
        #[response(status = 500)]
        Err(E)
    }
    // impl From<io::Result<File>> for Outcome<Html<File>, io::Error> {
    //     fn from(value: io::Result<File>) -> Self {
    //         match value {
    //             Ok(file) => Self::Ok(Html(file)),
    //             Err(error) => Self::Err(error)
    //         }
    //     }
    // }


    // #[get("/")]
    // async fn index(users: &State<db::Users>) -> Outcome<Html<File>, io::Error> {
    //     // Only allow if there is no admin user
    //     if helpers::admin_user_exists(users).await {
    //         return Outcome::Forbidden(())
    //     }
    //     File::open(RENDER_ROOT.join("admin-register.html")).await.into()
    // }
    #[get("/")]
    async fn index(users: &State<db::Users>, error: Option<FlashMessage<'_>>) -> Result<Html<TextStream![String]>, Forbidden<()>> {
        // Only allow if there is no admin user
        if helpers::admin_user_exists(users).await {
            return Err(Forbidden(None))
        }
        Ok(Html(TextStream(crate::components::render::<crate::components::authenticate::AdminRegister>(error.into()))))
    }

    #[post("/", data="<password>")]
    async fn admin_register(users: &State<db::Users>, password: Form<&str>) -> Outcome<Redirect, Flash<Redirect>> {
        // Only allow if there is no admin user
        if helpers::admin_user_exists(users).await {
            return Outcome::Forbidden(())
        }
        // Do not give the newly registered admin a session
        if let Err(error) = users.add_user(db::ADMIN_USR_ID, *password).await {
            return Outcome::Err(Flash::error(Redirect::to("/admin-register"), error.to_string()))
        }
        // Allow the user to log in separately
        Outcome::Ok(Redirect::to("/login"))
    }

    pub fn routes() -> Vec<Route> {
        routes![index, admin_register]
    }
}
