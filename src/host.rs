use std::path::Path;
use std::sync::Arc;
use std::task::{Context, Poll};

use pin_project::pin_project;

use axum::body::Body;

use http::{Request, Response, header};

use tower::{Layer, Service};

use tracing::{info, trace};

use super::AppState;
use super::jail::JailFuture;

#[pin_project(project = KindProj)]
enum Kind<F> {
    Normal(#[pin] F),
    Jail(#[pin] JailFuture),
}

#[pin_project]
pub struct ResponseFuture<F> {
    #[pin]
    inner: Kind<F>,
}

impl<F> ResponseFuture<F> {
    pub fn new_normal(fut: F) -> Self {
        Self {
            inner: Kind::Normal(fut),
        }
    }

    pub fn new_deny_file<P: AsRef<Path>>(path: P, req: Request<Body>) -> Self {
        Self {
            inner: Kind::Jail(JailFuture::new_deny_file(path, req)),
        }
    }

    pub fn new_deny_text() -> Self {
        Self {
            inner: Kind::Jail(JailFuture::new_deny_text()),
        }
    }

    pub fn new_deny() -> Self {
        Self {
            inner: Kind::Jail(JailFuture::new_deny()),
        }
    }
}

impl<F, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<Body>, E>>,
{
    type Output = Result<Response<Body>, E>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().inner.project() {
            KindProj::Normal(fut) => fut.poll(cx),
            KindProj::Jail(fut) => fut.poll(cx).map(|x| Ok(x.unwrap())),
        }
    }
}

#[derive(Clone)]
pub struct HostCheck<S> {
    state: Arc<AppState>,
    inner: S,
}

impl<S> HostCheck<S> {
    pub fn new(inner: S, state: Arc<AppState>) -> Self {
        Self { inner, state }
    }

    fn do_jail<F>(&self, req: Request<Body>) -> ResponseFuture<F> {
        match self.state.get_jail_file() {
            Some(path) => ResponseFuture::new_deny_file(path, req),
            None => ResponseFuture::new_deny_text(),
        }
    }
}

impl<S> Service<Request<Body>> for HostCheck<S>
where
    S: Service<Request<Body>, Response = Response<Body>>,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let headers = req.headers();
        let path = req.uri().path();

        // just junk random images or various things with 418
        if path == "/favicon.ico" {
            trace!("favicon.ico, discarding");
            return ResponseFuture::new_deny();
        }

        // deny known plaintext files
        if path.ends_with(".env") {
            trace!("denying plaintext file {}", path);
            return ResponseFuture::new_deny_text();
        }

        if let Some(host) = headers.get(header::HOST) {
            let host = host.to_str().unwrap_or_default();

            if host != self.state.host {
                // skip over well-known and robots.txt because bots are supposed to read this one
                if !path.starts_with("/.well-known") && path != "/robots.txt" {
                    trace!("jail hit for path {}, host {}", path, host);
                    return self.do_jail(req);
                }
            }
        }

        // junk or weird paths to get around bad filtering (ironic being said here)
        if path.starts_with("//") {
            trace!("jail hit for malformed path {}", path);
            return self.do_jail(req);
        }

        // nice try but this isn't wordpress
        if path.starts_with("/wp-") || path.ends_with(".php") {
            trace!("jail hit for wordpress requests path {}", path);
            return self.do_jail(req);
        }

        // TODO: don't hardcode this
        if !path.starts_with("/static")
            && path != "/car"
            && path != "/"
            && path != "/art"
            && path != "/about"
            && path != "/.well-known/matrix/client"
            && path != "/.well-known/matrix/server"
            && path != "/robots.txt"
            && path != "/jail"
        {
            info!("strange path request: {}", path);
        } else {
            trace!("path request: {}", path);
        }

        ResponseFuture::new_normal(self.inner.call(req))
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
}

#[derive(Clone)]
pub struct HostCheckLayer {
    state: Arc<AppState>,
}

impl HostCheckLayer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for HostCheckLayer {
    type Service = HostCheck<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HostCheck::new(inner, self.state.clone())
    }
}
