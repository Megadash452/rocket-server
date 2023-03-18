Hello, I'm using rocket.rs to make an http server, and I'm getting this error when trying to send some type of stream. For some reason the error only happens for this specific fn and not others like it. I've tried to look up this error but it seems this error is for recursive async fns, but this fn is not recursive.
The error goes away when the fn is not async, but for my purposes it will propbably need to be async.

```rs
async fn index_md(user: Option<auth::User>) -> RawHtml<TextStream<impl async_std::stream::Stream<Item=String>>> {
  RawHtml(TextStream(components::render::<components::Document>(
    components::DocProps {
      header: user.into(),
      content: markdown::to_html(&std::fs::read_to_string("./routes/index.md").expect("no index file"))
    }
  )))
}
```
```
error[E0391]: cycle detected when borrow-checking `index_md`
  --> src/main.rs:47:113
   |
47 |   async fn index_md(user: Option<auth::User>) -> RawHtml<TextStream<impl async_std::stream::Stream<Item=String>>> {
   |  _________________________________________________________________________________________________________________^
48 | |     RawHtml(TextStream(components::render::<components::Document>(
49 | |         components::DocProps {
50 | |             header: user.into(),
...  |
54 | |     )))
55 | | }
   | |_^
   |
   = note: ...which requires evaluating trait selection obligation `<[async fn body@src/main.rs:47:113: 55:2] as core::future::future::Future>::Output == rocket::response::content::RawHtml<rocket::response::stream::text::TextStream<impl futures_core::stream::Stream<Item = alloc::string::String>>>`...
note: ...which requires computing type of `index_md::{opaque#0}::{opaque#0}`...
  --> src/main.rs:47:67
   |
47 | async fn index_md(user: Option<auth::User>) -> RawHtml<TextStream<impl async_std::stream::Stream<Item=String>>> {
   |                                                                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: ...which again requires borrow-checking `index_md`, completing the cycle
note: cycle used when computing type of `index_md::{opaque#0}`
  --> src/main.rs:47:48
   |
47 | async fn index_md(user: Option<auth::User>) -> RawHtml<TextStream<impl async_std::stream::Stream<Item=String>>> {
   |                                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

This other fn does not get the error
```rs
async fn index_admin(_admin: Admin, error: Option<FlashMessage<'_>>) -> Html<TextStream<impl async_std::stream::Stream<Item=String>>> {
        Html(TextStream(crate::components::render::<crate::components::authenticate::Register>(error.into())))
    }
```
