use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context as Cx, Poll};
use std::time::{Duration, Instant};

use pin_project::pin_project;

use axum::body::Body;

use http::{Request, Response, header};

use tower::{Layer, Service};

use tracing::{error, info, trace, warn};

use anyhow::Context;

use rand::prelude::*;

use crate::jail::JailFuture;

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

#[pin_project(project = KindProj)]
enum Kind<F> {
    Normal(#[pin] F),
    Jail(#[pin] super::jail::JailFuture),
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

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Cx<'_>) -> Poll<Self::Output> {
        match self.project().inner.project() {
            KindProj::Normal(fut) => fut.poll(cx),
            KindProj::Jail(fut) => fut.poll(cx).map(|x| Ok(x.unwrap())),
        }
    }
}

#[derive(Clone)]
pub struct HostCheck<S> {
    host: String,
    files: Vec<PathBuf>,
    time: TimeSync,
    inner: S,
}

impl<S> HostCheck<S> {
    pub fn new(inner: S, host: String, files: Vec<PathBuf>, time: TimeSync) -> Self {
        Self {
            inner,
            host,
            files,
            time,
        }
    }

    fn do_jail<F>(&self, req: Request<Body>) -> ResponseFuture<F> {
        let len = self.files.len();

        // attempting to chose a random with a range from 0 to 0 panics so early return
        if len == 0 {
            warn!("no files to send, returning default response");
            return ResponseFuture::new_deny_text();
        }

        // lock the mutex to access the last index change instant, then determine if a new file
        // should be used for the jail
        {
            let cur = Instant::now();
            let mut last_change = match self.time.last_change.lock() {
                Ok(l) => l,
                // TODO: if the mutex is poisoned deal with this correctly
                Err(_) => {
                    warn!("failed to lock mutex, returning default response");
                    return ResponseFuture::new_deny_text();
                }
            };

            if *last_change + Duration::from_secs(30) < cur {
                trace!("longer than 30s since last media change, changing");

                let mut rng = rand::rng();
                *last_change = cur;
                self.time
                    .cur_file
                    .store(rng.random_range(..len), Ordering::SeqCst);
            }
        }

        let path = match self.files.get(self.time.cur_file.load(Ordering::SeqCst)) {
            Some(p) => p,
            None => {
                warn!("failed to get file path, returning default response");
                return ResponseFuture::new_deny_text();
            }
        };

        ResponseFuture::new_deny_file(path, req)
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

        // TODO: Seperate this out when the random file part gets sent into its own service, this
        // needs to be in the main axum router instead of hardcoded here
        if path == "/jail" {
            trace!("intended jail link, sending jail response");
            return self.do_jail(req);
        }

        if let Some(host) = req.headers().get(header::HOST) {
            let host = host.to_str().unwrap_or_default();

            if host != self.host {
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
        {
            info!("strange path request: {}", path);
        } else {
            trace!("path request: {}", path);
        }

        ResponseFuture::new_normal(self.inner.call(req))
    }

    fn poll_ready(&mut self, cx: &mut Cx<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
}

#[derive(Clone)]
pub struct HostCheckLayer {
    time: TimeSync,
    host: String,
    files: Vec<PathBuf>,
}

impl HostCheckLayer {
    pub fn new() -> anyhow::Result<Self> {
        let files = Self::read_dir().unwrap_or_default();
        let len = files.len();
        let time = TimeSync::new();

        // manually run the rng once, this instance is cloned across and the server should start
        // with a random file at first, if there was no files found, then HostCheck will fall back
        // to default text, check len manually because random will panic with a range of 0 to 0
        if len != 0 {
            trace!("initializing first file");
            time.cur_file
                .store(rand::rng().random_range(..len), Ordering::SeqCst);
        } else {
            error!("failed to read any files in static/jail, will only return basic deny");
        }

        Ok(Self {
            host: std::env::var("WEBSITE_HOST").context("failed to get WEBSITE_HOST")?,
            time,
            files,
        })
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
        HostCheck::new(
            inner,
            self.host.clone(),
            self.files.clone(),
            self.time.clone(),
        )
    }
}
