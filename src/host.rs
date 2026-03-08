use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use pin_project::pin_project;

use axum::{body::Body, response::IntoResponse};

use http::{HeaderValue, Request, Response, StatusCode, header};

use tower::util::Oneshot;
use tower::{Layer, Service};
use tower_http::services::ServeFile;

use rand::prelude::*;

#[derive(Clone)]
pub struct TimeSync {
    last_change: Arc<Mutex<Instant>>,
    cur_file: Arc<AtomicUsize>,
}

impl TimeSync {
    pub fn new() -> Self {
        Self {
            last_change: Arc::new(Mutex::new(Instant::now())),
            cur_file: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[derive(Clone)]
pub struct HostCheck<S> {
    files: Vec<PathBuf>,
    time: TimeSync,
    inner: S,
}

impl<S> HostCheck<S> {
    pub fn new(inner: S, files: Vec<PathBuf>, time: TimeSync) -> Self {
        Self { inner, files, time }
    }

    fn do_jail<F>(&self, req: Request<Body>) -> ResponseFuture<F> {
        let len = self.files.len();

        // attempting to chose a random with a range from 0 to 0 panics so early return
        if len == 0 {
            return ResponseFuture::new_deny_text();
        }

        // lock the mutex to access the last index change instant, then determine if a new file
        // should be used for the jail
        {
            let cur = Instant::now();
            let mut last_change = match self.time.last_change.lock() {
                Ok(l) => l,
                // TODO: if the mutex is poisoned deal with this correctly
                Err(_) => return ResponseFuture::new_deny_text(),
            };

            if *last_change + Duration::from_secs(30) < cur {
                let mut rng = rand::rng();
                *last_change = cur;
                self.time
                    .cur_file
                    .store(rng.random_range(..len), Ordering::SeqCst);
            }
        }

        let path = match self.files.get(self.time.cur_file.load(Ordering::SeqCst)) {
            Some(p) => p,
            None => return ResponseFuture::new_deny_text(),
        };

        ResponseFuture::new_deny_file(path, req)
    }
}

#[pin_project(project = KindProj)]
enum Kind<F> {
    Normal(#[pin] F),
    DenyFile(#[pin] Oneshot<ServeFile, Request<Body>>),
    DenyText,
    Deny,
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
            inner: Kind::DenyFile(Oneshot::new(ServeFile::new(path), req)),
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

impl<F, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<Body>, E>>,
{
    type Output = Result<Response<Body>, E>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().inner.project() {
            KindProj::Normal(fut) => fut.poll(cx),
            KindProj::DenyFile(file) => file.poll(cx).map(|f| {
                let mut res = f
                    .expect("ServeFile should not error, it is infallible, please check")
                    .into_response();

                // ServeFile assumes all the text is ascii, but thats very old and no fun, so fix
                // all text/plain responses to correctly set the charset to utf-8
                if let Some(ct) = res.headers().get(header::CONTENT_TYPE) {
                    if ct == "text/plain" {
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/plain; charset=utf-8"),
                        );
                    }
                }

                res.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

                return Ok(res);
            }),
            KindProj::DenyText => {
                Poll::Ready(Ok(Response::new("rm -rf --no-preserve root /".into())))
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

impl<S> Service<Request<Body>> for HostCheck<S>
where
    S: Service<Request<Body>, Response = Response<Body>>,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let path = req.uri().path();

        if path == "/favicon.ico" {
            return ResponseFuture::new_deny();
        }

        // TODO: Seperate this out when the random file part gets sent into its own service, this
        // needs to be in the main axum router instead of hardcoded here
        if path == "/jail" {
            return self.do_jail(req);
        }

        if let Some(host) = req.headers().get(header::HOST) {
            if !host
                .to_str()
                .unwrap_or_default()
                .starts_with("aamaruvi.com")
            {
                // skip over well-known because bots are supposed to read this one
                if !path.starts_with("/.well-known") {
                    return self.do_jail(req);
                }
            }
        }

        ResponseFuture::new_normal(self.inner.call(req))
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
}

#[derive(Clone)]
pub struct HostCheckLayer {
    time: TimeSync,
    files: Vec<PathBuf>,
}

impl HostCheckLayer {
    pub fn new() -> Self {
        let this = Self {
            time: TimeSync::new(),
            files: Self::read_dir().unwrap_or_default(),
        };

        // manually run the rng once, this instance is cloned across and the server should start
        // with a random file at first, if there was no files found, then HostCheck will fall back
        // to default text, check len manually because random will panic with a range of 0 to 0
        let len = this.files.len();
        if len != 0 {
            this.time
                .cur_file
                .store(rand::rng().random_range(..len), Ordering::SeqCst);
        }

        this
    }

    fn read_dir() -> std::io::Result<Vec<PathBuf>> {
        std::fs::read_dir("static/jail")?
            .map(|res| res.map(|e| e.path()))
            .collect()
    }
}

impl<S> Layer<S> for HostCheckLayer {
    type Service = HostCheck<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HostCheck::new(inner, self.files.clone(), self.time.clone())
    }
}
