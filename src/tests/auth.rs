//! There can only be one test : (

use crate::rocket_server;
use std::process::Command;
use rocket::{
    http::{Status, ContentType},
    local::blocking::Client,
};

static TEST_ROOT: &str = "/tmp/rocket-server-test";

/// Tests for `this` to be equal (by [`PartialEq::eq()`]) to one of `others`.
fn eq_one_of<O, T: PartialEq<O>>(this: T, others: impl AsRef<[O]>) -> bool {
    for item in others.as_ref() {
        if this == *item {
            return true
        }
    }

    false
}
#[inline]
/// Opposite of [`eq_one_of()`].
fn eq_none_of<O, T: PartialEq<O>>(this: T, others: impl AsRef<[O]>) -> bool {
    !eq_one_of(this, others)
}


fn client() -> Client {
    // clear test-root dir
    #[allow(unused_must_use)] {
    Command::new("rm")
        .arg(TEST_ROOT)
        .output();
    }
    Command::new("mkdir")
        .arg("-p")
        .arg(TEST_ROOT)
        .output().expect("Can't create test-root dir");

    std::fs::read_dir(".").unwrap()
        .filter_map(|entry|
            entry.ok()
                .and_then(|entry| Some(entry.path()))
        )
        .filter(|path| {
            let path = path.as_os_str();
            eq_none_of(path, ["src", "target", "secrets", "Cargo.toml", "Cargo.lock"])
        }).for_each(|entry|{
            Command::new("cp")
                .arg(entry)
                .arg(TEST_ROOT)
                .output().unwrap();
        });

    std::env::set_current_dir(TEST_ROOT).expect("Can't set cwd");

    Client::tracked(rocket_server())
        .expect("Can't create Rocket instance")
}

#[test]
/// There can only be one test : (
fn auth() {
    let client = client();
    let mut response;
    
    // /login and /register should lead to /admin-register because the admin user does not exist.
    response = client.get("/login").dispatch();
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/admin-register"));
    response = client.get("/register").dispatch();
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/admin-register"));

    // Set up admin user so we are able to go to /login and /register.
    response = client.get("/admin-register").dispatch();
    assert_eq!(response.status(), Status::Ok);
    // Send (POST) new password for admin
    response = client.post("/admin-register")
        .header(ContentType::Form)
        .body("password=password")
        .dispatch();
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/login"));

    // Log in as admin
    response = client.get("/login").dispatch();
    assert_eq!(response.status(), Status::Ok);
    // Send (POST) user credentials
    response = client.post("/login")
        .header(ContentType::Form)
        .body("username=admin&password=password")
        .dispatch();
    // Check the client was sent a "session_uuid" cookie
    assert!(client.cookies().get(crate::auth::SESSION_COOKIE).is_some());
    // After logging in, Redirects to root (/)
    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/"));

    // Admin can register new users
    response = client.get("/register").dispatch();
    assert_eq!(response.status(), Status::Ok);
    // TODO

    // Non-users can't register new users

    // ? Other users can't register new users??
}
