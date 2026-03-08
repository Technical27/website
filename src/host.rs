use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pin_project::pin_project;

use axum::{body::Body, response::IntoResponse};

use http::{HeaderValue, Request, Response, header};

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
}

#[pin_project(project = KindProj)]
enum Kind<F> {
    Normal(#[pin] F),
    DenyFile(#[pin] Oneshot<ServeFile, Request<Body>>),
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

    pub fn new_deny() -> Self {
        Self { inner: Kind::Deny }
    }
}

impl<F, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<Body>, E>>,
{
    type Output = Result<Response<Body>, E>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project().inner.project() {
            KindProj::Normal(fut) => fut.poll(cx),
            KindProj::DenyFile(file) => file.poll(cx).map(|f| {
                let mut res = f
                    .expect("ServeFile should not return an error")
                    .into_response();

                // serve files assumes all the text is ascii, but thats very old and no fun, so fix
                // all text/plain responses to correctly set the charset to utf-8
                if let Some(ct) = res.headers().get(header::CONTENT_TYPE) {
                    if ct == "text/plain" {
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/plain; charset=utf-8"),
                        );
                    }
                }

                return Ok(res);
            }),
            KindProj::Deny => {
                std::task::Poll::Ready(Ok(Response::new("rm -rf --no-preserve root /".into())))
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
        if let Some(host) = req.headers().get(header::HOST) {
            if host == "localhost:3000" {
                if req.uri() == "/favicon.ico" {
                    return ResponseFuture::new_deny();
                }

                {
                    let cur = Instant::now();
                    let mut last_change = self.time.last_change.lock().unwrap();

                    if *last_change + Duration::from_secs(10) < cur {
                        println!("time change");
                        let mut rng = rand::rng();
                        let len = self.files.len();
                        *last_change = cur;
                        self.time
                            .cur_file
                            .store(rng.random_range(..len), Ordering::SeqCst);
                    }
                }

                let path = match self.files.get(self.time.cur_file.load(Ordering::SeqCst)) {
                    Some(p) => p,
                    None => return ResponseFuture::new_deny(),
                };

                return ResponseFuture::new_deny_file(path, req);
            }
        }

        ResponseFuture::new_normal(self.inner.call(req))
    }

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
}

#[derive(Clone)]
pub struct HostCheckLayer {
    time: TimeSync,
}

impl HostCheckLayer {
    pub fn new() -> Self {
        Self {
            time: TimeSync::new(),
        }
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
        HostCheck::new(inner, Self::read_dir().unwrap(), self.time.clone())
    }
}
