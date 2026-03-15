use std::convert::Infallible;
use std::path::Path;
use std::task::{Context as Cx, Poll};

use pin_project::pin_project;

use axum::{body::Body, response::IntoResponse};

use http::{HeaderValue, Request, Response, StatusCode, header};

use tower::util::Oneshot;
use tower_http::services::ServeFile;

#[pin_project(project = KindProj)]
enum Kind {
    DenyFile(#[pin] Box<Oneshot<ServeFile, Request<Body>>>),
    DenyText,
    Deny,
}

#[pin_project]
pub struct JailFuture {
    #[pin]
    inner: Kind,
}

impl JailFuture {
    pub fn new_deny_file<P: AsRef<Path>>(path: P, req: Request<Body>) -> Self {
        Self {
            inner: Kind::DenyFile(Box::new(Oneshot::new(ServeFile::new(path), req))),
        }
    }

    pub fn new_deny_text() -> Self {
        Self {
            inner: Kind::DenyText,
        }
    }

    pub fn new_deny() -> Self {
        Self { inner: Kind::Deny }
    }
}

impl Future for JailFuture {
    type Output = Result<Response<Body>, Infallible>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Cx<'_>) -> Poll<Self::Output> {
        match self.project().inner.project() {
            KindProj::DenyFile(file) => file.poll(cx).map(|f| {
                let mut res = f
                    .expect("ServeFile should not error, it is infallible, please check")
                    .into_response();

                let headers = res.headers_mut();

                // ServeFile assumes all the text is ascii, but thats very old and no fun, so fix
                // all text/plain responses to correctly set the charset to utf-8
                if let Some(ct) = headers.get(header::CONTENT_TYPE)
                    && ct == "text/plain"
                {
                    headers.insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("text/plain; charset=utf-8"),
                    );
                }

                // don't let the browser store any response to avoid weird behavior
                headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                // tell to display inline, hopefully fixes videos to properly play
                headers.insert(
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_static("inline"),
                );

                return Ok(res);
            }),
            KindProj::DenyText => {
                Poll::Ready(Ok(Response::new("rm -rf --no-preserve root /\n\n".into())))
            }
            KindProj::Deny => {
                let res = Response::builder()
                    .status(StatusCode::IM_A_TEAPOT)
                    .body("I'm a Teapot!".into())
                    .expect("hard coded http response fail");
                Poll::Ready(Ok(res))
            }
        }
    }
}
