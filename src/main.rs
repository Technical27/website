mod host;
mod jail;

use axum::extract::{Request, State};
use axum::response::Response;
use axum::{Router, extract::ConnectInfo, response::Html, routing::get};

use axum::Json;
use http::{HeaderValue, header};
use serde_json::{Value, json};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::env;

use askama::Template;

use rand::prelude::*;

use anyhow::{Context, Result, anyhow};
use axum_anyhow::ApiResult;

use tracing::{Level, error, trace, warn};
use tracing_subscriber::FmtSubscriber;

use jail::JailFuture;

pub struct AppState {
    last_change: Mutex<Instant>,
    cur_file: AtomicUsize,
    filenames: Vec<PathBuf>,
    host: String,
}

impl AppState {
    pub fn get_jail_file(&self) -> Option<&PathBuf> {
        let len = self.filenames.len();

        // attempting to chose a random with a range from 0 to 0 panics so early return
        if len == 0 {
            trace!("no files to send, returning default response");
            return None;
        }

        // lock the mutex to access the last index change instant, then determine if a new file
        // should be used for the jail
        {
            let cur = Instant::now();
            let mut last_change = match self.last_change.lock() {
                Ok(l) => l,
                // TODO: if the mutex is poisoned deal with this correctly
                Err(_) => {
                    warn!("failed to lock mutex, returning default response");
                    return None;
                }
            };

            if *last_change + Duration::from_secs(30) < cur {
                trace!("longer than 30s since last media change, changing");

                let mut rng = rand::rng();
                *last_change = cur;
                self.cur_file
                    .store(rng.random_range(..len), Ordering::SeqCst);
            }
        }

        self.filenames.get(self.cur_file.load(Ordering::SeqCst))
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct RootTemplate<'a> {
    title: &'a str,
    source: &'a SocketAddr,
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate<'a> {
    title: &'a str,
}

#[derive(Template)]
#[template(path = "art.html")]
struct ArtTemplate<'a> {
    title: &'a str,
}

// TODO: move this out of main
const MOTD: &'static [&'static str] = &[
    "0x41414141",
    "48/00/1980",
    "21522",
    "5555",
    "1970-01-01",
    "9.99999999999E499",
    "3.14159265",
    "0xbada55",
    "0xcafebabe",
    "8008135",
    "67",
    "int3",
    "440hz",
    "f/4",
    "SWING!",
    "WHO THE FUCK IS OCTAVE",
    "N52",
    "48khz/24bit",
    "mls",
    "LO SIENTO",
    "WILSON",
    "so i went to the coinstar",
    "it didn't have any coins",
    "slam storage",
    "batabatabatabatabatabatabatabatabatabatabatabatabatabatabatabatabatabatabatabata",
    "tch... newgens...",
    "jamie paige is mid af",
    "all caps or no caps",
    "-8.892098 kohm",
    "MUSTARRRRRDDDDD",
    "undefined",
    "Segmentation fault",
    "TEMP('C) -1583.1",
    "hello james. yes, YOU🫵",
    "Shit man, this beat league is fucked.",
    "let mut suvi: u64 = 0;",
    "143",
    "iPod mini",
    "HP 48G+",
    "E90",
    "4117537",
    "Half-Life: 4",
    "HAMMOOOONNNDDDD",
    "YOU'VE REVERSED INTO THE SPORTS LORRY",
    "2240620",
    "You're Unbeatable!",
    "vivid/stasis",
    "AAAAHHHHHHH I NEEEEEED A MEDIC BAG",
    "e^(pi * i) = -1",
    "30040",
    "M539",
    "FULL BRIDGE RECTIFIER",
    "🤼‍♀️",
    "I'm Quaving",
    "all hail boobs and beating off",
    "i'm so fucking TIIIIIRED",
    "eating is living",
    "you say the lines",
    "the syllables",
    "got it memorized so i can make it stop",
    "rest up! god im tired",
    "When you can't even say my name",
    "Finished `dev` profile [unoptimized + debuginfo] targets(s) in 0.69s",
    "Hey you, you're finally awake",
    "George Washington",
    "ZIG SILAS",
    "steamdeck",
    "calc is short for calculator",
    "titanfall was the best shooter",
    "the LEGO movie is amazing",
    "huge boob",
    "octave",
    "breast is good",
    "man door hand hook car door",
    "Stealy Wheely Automobiley",
    "One day you will be dead.",
    "ROCK AND STONE",
    "WE'RE RICH",
    "Have an Easy day!",
    "A HIDEO KOJIMA PRODUCTION",
    "3 babies 1 minivan",
    "You just lost the game.",
    "CorrectHorseBatteryStaple",
    "The 15th Standard",
    "🔰",
    "god dammed tired.",
    "memorizer",
    "featuring",
    "DOOR STUCK DOOR STUCK",
    "PLEASE I BEG YOU",
    "PERFECT+31",
    "wiggle your fingers, jam the keys",
    "::<>",
    "5/8in",
    "K-POP!",
    "21",
    "how to quit vim?",
    "SCSI",
    "FORK FOUND IN KITCHEN",
    "THE NUMBERS MASON",
    "WHAT DO THEY MEAN",
    "Press F to Pay Respects",
    "Soap, what the hell kinda name is Soap?",
    "Turbofish",
    "Wayland is great",
    "WBA",
    "WBS",
    "3MW",
    "5UM",
    "4US",
    "KWebsiteTitle",
    "Kandalf",
    "KDE is amazing",
    "Sway rocks",
    "Gentoo",
    "eselect news read",
    "cargo run --release",
    "i also have to look up song lyrics",
    "take backwards crowbar of the right",
    "how to use git tutorial 2026 working",
    "ava",
    "TheLegend27",
    "rating up! +0.000",
    "the best ragebait is vivid/stasis",
    "255/35R18",
    "hi bramble",
    "HTCPCP",
    "HTCPCP-TEA",
    "quabubu",
    "beepo",
    "LETS GO GAMBLING",
    "99% of gamblers quit before the god run",
    "what if jfk shot back",
    "judge judy and executioner",
    "women🔥",
    "crazy, i was crazy once",
    "they locked me in a room",
    "a rubber room with rats",
    "the rats made me crazy",
    "quazy, i was quazy once",
    "they locked me in a room",
    "a room with quaverlings",
    "the quaverlings made me quazy",
    "Quaver",
    "Beat",
    "Treble",
    "Clef",
    "SOI SOI SOI SOI",
    "nyancat",
    "man this audio is shit",
    "six seven",
    "im a dog meow meow",
    "🧲",
    "painting with dirt",
    "c418 - sweden",
    "Battle Tapes",
    "the world doesn't care",
    "carbon fiber towels",
    "LIQUIMOLY",
];

fn motd() -> Result<&'static str> {
    match MOTD.iter().choose(&mut rand::rng()) {
        Some(m) => Ok(m),
        None => Err(anyhow!("failed to choose motd")),
    }
}

async fn root(connection: ConnectInfo<SocketAddr>) -> ApiResult<Html<String>> {
    let root_template = RootTemplate {
        title: &motd()?,
        source: &connection,
    };
    Ok(Html(root_template.render()?))
}

async fn about() -> ApiResult<Html<String>> {
    let about_template = AboutTemplate { title: &motd()? };
    Ok(Html(about_template.render()?))
}

async fn art() -> ApiResult<Html<String>> {
    let art_template = ArtTemplate { title: &motd()? };
    Ok(Html(art_template.render()?))
}

async fn car() -> ApiResult<String> {
    Ok(motd()?.to_owned() + "\nunder construction, just use the back button")
}

// returns information about the matrix homeserver to any client
async fn matrix_client() -> Json<Value> {
    Json(json!({ "m.homeserver": { "base_url": "https://matrix.aamaruvi.com" } }))
}

// returns the matrix homeserver to any other homeserver to federate
async fn matrix_server() -> Json<Value> {
    Json(json!({ "m.server": "matrix.aamaruvi.com:443" }))
}

async fn robots() -> &'static str {
    "# if robot: beep boop beep beep boop\n# if human: hello there, please leave this is not your domain\nUser-agent: *\nDisallow: /\n"
}

fn read_dir() -> std::io::Result<Vec<PathBuf>> {
    std::fs::read_dir("static/jail")?
        .map(|res| res.map(|e| e.path()))
        .collect()
}

fn init_state() -> Result<Arc<AppState>> {
    let files = read_dir()?;
    let len = files.len();

    let idx = if len == 0 {
        error!("failed to read any files, jail will be not functional");
        0
    } else {
        rand::rng().random_range(..len)
    };

    Ok(Arc::new(AppState {
        cur_file: AtomicUsize::new(idx),
        last_change: Mutex::new(Instant::now()),
        filenames: files,
        host: env::var("WEBSITE_HOST").context("failed to get WEBSITE_HOST")?,
    }))
}

async fn jail(state: State<Arc<AppState>>, req: Request) -> Response {
    trace!("intended jail link, sending jail response");

    match state.get_jail_file() {
        Some(p) => JailFuture::new_deny_file(p, req),
        None => JailFuture::new_deny_text(),
    }
    .await
    .unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let level = env::var("WEBSITE_LOG_LEVEL")
        .context("failed to get env WEBSITE_LOG_LEVEL")
        .and_then(|x| Level::from_str(&x).context("failed to parse WEBSITE_LOG_LEVEL"))
        .unwrap_or(Level::INFO);

    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let state = init_state()?;

    let app = Router::new()
        .route("/", get(root))
        .route("/about", get(about))
        .route("/art", get(art))
        .route("/car", get(car))
        .route("/.well-known/matrix/client", get(matrix_client))
        .route("/.well-known/matrix/server", get(matrix_server))
        .route("/robots.txt", get(robots))
        .route("/jail", get(jail))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/.well-known", ServeDir::new(".well-known"))
        // Set no-cache due to many dyanmic things on all parts of the website
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        // Internally sets no-store when needed
        .layer(host::HostCheckLayer::new(state.clone()))
        .with_state(state);

    let listen_addr =
        env::var("WEBSITE_BIND_ADDR").context("failed to read WEBSITE_BIND_ADDR")?;

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
