#![feature(path_trailing_sep)]

mod host;
mod jail;

use axum::extract::{Request, State, Path};
use axum::response::Response;
use axum::{Extension, Router, response::Html, routing::get};

use axum::Json;
use http::{HeaderValue, StatusCode, header};
use serde_json::{Value, json};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use askama::Template;

use rand::prelude::*;

use anyhow::{Context, Result};
use axum_anyhow::{ApiError, ApiResult, ResultExt};

use tracing::{Level, error, trace, warn};
use tracing_subscriber::FmtSubscriber;

use jail::JailFuture;

pub struct AppState {
    last_change: Mutex<Instant>,
    cur_file: AtomicUsize,
    filenames: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct AppConfig {
    host: String,
    rpoxy: bool,
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
    messages: &'a [&'a str],
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

#[derive(Template)]
#[template(path = "smart.html")]
struct SmartTemplate<'a> {
    title: &'a str,
}

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate<'a> {
    title: &'a str,
}

// TODO: move this out of main
const MOTD: &[&str] = &[
    // ASCII AAAA in hexadecimal
    "0x41414141",
    // funny ibm bios error from clab retro video
    "48/00/1980",
    // personal ;)
    "21522",
    "5555",
    // unix time epoch
    "1970-01-01",
    // hp48 max real
    "9.99999999999E499",
    // some digits of pi
    "3.14159265",
    // kerbal green
    "0xbada55",
    // funny word/numbers
    "0xcafebabe",
    "8008135",
    "67",
    // x86 asm interrupt instruction
    "int3",
    // frequency of an A above middle C
    "440hz",
    // camera aperture
    "f/4",
    "SWING!",
    "WHO THE FUCK IS OCTAVE",
    // BMW engine
    "N52",
    "48kHz/24bit",
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
    // Marco Reps Fluke 8508A repair
    "-8.892098 kohm",
    "MUSTARRRRRDDDDD",
    "undefined",
    "Segmentation fault",
    // Marco Reps HP3458A repairathon
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
    // memorized
    "eating is living",
    "you say the lines",
    "the syllables",
    "got it memorized so i can make it stop",
    "rest up! god im tired",
    // INVISIBLE
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
    // Deep Rock Galactic
    "ROCK AND STONE",
    "WE'RE RICH",
    // Easy Delivery Company
    "Have an Easy day!",
    "A HIDEO KOJIMA PRODUCTION",
    "3 babies 1 minivan",
    "You just lost the game.",
    "CorrectHorseBatteryStaple",
    "The 15th Standard",
    // Japanese Symbol for Beginner.
    "🔰",
    // UNBEATABLE arcade titles
    "god dammed tired.",
    "memorizer",
    "featuring",
    // famous
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
    // famous CoD lines
    "THE NUMBERS MASON",
    "WHAT DO THEY MEAN",
    "Press F to Pay Respects",
    "Soap, what the hell kinda name is Soap?",
    "Turbofish",
    "Wayland is great",
    // VIN WMIs for BMW
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
    // ava
    "ava",
    // amazing mobile game ad
    "TheLegend27",
    "rating up! +0.000",
    // i wrote this before even making it to chapter 6 or doing encore
    "the best ragebait is vivid/stasis",
    // car tires are measured in such a dumb way
    "255/35R18",
    "hi bramble",
    // RFC april fools joke
    "HTCPCP",
    "HTCPCP-TEA",
    "quabubu",
    "beepo",
    "LETS GO GAMBLING",
    "99% of gamblers quit before the god run",
    "what if jfk shot back",
    "judge judy and executioner",
    "women🔥",
    // common one
    "crazy, i was crazy once",
    "they locked me in a room",
    "a rubber room with rats",
    "the rats made me crazy",
    // common one with changed text
    "quazy, i was quazy once",
    "they locked me in a room",
    "a room with quaverlings",
    "the quaverlings made me quazy",
    // UNBEATABLE is
    "Quaver",
    "Beat",
    "Treble",
    "Clef",
    "Rest",
    "and YOU",
    // some classics
    "SOI SOI SOI SOI",
    "nyancat",
    "man this audio is shit",
    "six seven",
    "im a dog meow meow",
    "🧲",
    "painting with dirt",
    "c418 - sweden",
    "Battle Tapes",
    // usedcvnt
    "the world doesn't care",
    "carbon fiber towels",
    "LIQUIMOLY",
    "window.location.href = \"/i/am/very/smart\"",
    "its some good schiit",
    // cd quality is damn good
    "44.1kHz/16bit is peak audio",
    // re-tuned
    "when i lose my mind",
    "will you help me find it",
    "or will you leave me to rot",
    // famous song
    "you know the rules",
    "and so do i",
    "never gonna give you up",
    // shhh
    "/i/am/very/smart",
    "Prepare for Titanfall",
    "dt770s are amazing",
    "RS-232",
    "its actually DE-9 and DB-25",
    "pipewire is magic",
    // Where Wayland got its name
    "Wayland, Massachusetts",
    // BMW engine
    "S65B40",
    "S62B50",
    // Ohio
    "Ohio",
    "ANYWHERE BUT OHIO",
    // ISO film speed is a combo of ASA and DIN but most just leave the DIN part out
    "ISO = ASA/DIN",
];

fn motd() -> ApiResult<&'static str> {
    match MOTD.iter().choose(&mut rand::rng()) {
        Some(m) => Ok(m),
        None => Err(ApiError::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .title("Internal Server Error")
            .detail("motd machine broke ask again tomorrow")
            .build()),
    }
}

type HtmlTemplate = ApiResult<Html<String>>;

fn render_template(template: &impl Template) -> HtmlTemplate {
    template
        .render()
        .context_internal("Internal Server Error", "Failed to work right idk")
        .map(Html)
}

async fn root(src: Extension<IpAddr>) -> HtmlTemplate {
    let mut msgs = Vec::new();
    if src.is_ipv6() {
        msgs.push("you are connected over IPv6 and participating in the transition away from a fundamentally broken internet");
    }

    if rand::rng().random_ratio(1, 20) {
        msgs.push("something is at /i/am/very/smart");
    }

    render_template(&RootTemplate {
        title: motd()?,
        messages: &msgs,
    })
}

async fn about() -> HtmlTemplate {
    render_template(&AboutTemplate { title: motd()? })
}

async fn art() -> HtmlTemplate {
    render_template(&ArtTemplate { title: motd()? })
}

async fn car() -> ApiResult<String> {
    Ok(motd()?.to_owned() + "\nunder construction, just use the back button")
}

async fn contact() -> HtmlTemplate {
    render_template(&ContactTemplate { title: motd()? })
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

async fn idiot() -> HtmlTemplate {
    render_template(&SmartTemplate { title: motd()? })
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
    }))
}

fn init_config() -> Result<AppConfig> {
    Ok(AppConfig {
        host: env::var("WEBSITE_HOST").context("failed to get WEBSITE_HOST")?,
        rpoxy: env::var_os("WEBSITE_RPROXY").is_some(),
    })
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


#[derive(Template)]
#[template(path = "blog.html")]
struct BlogTemplate<'a> {
    motd: &'a str,
    markdown: String,
}

async fn blog_render(Path(mut md_path): Path<std::path::PathBuf>) -> HtmlTemplate {
    // XXX: check if this actually could be possible
    if md_path.is_absolute() {
                return Err(anyhow::anyhow!("error").into());
    }

    println!("path: {:?}", md_path);
    if md_path.has_trailing_sep() {
        md_path.push("index.md");
    } else {
        md_path.set_extension("md");
    }

    let md_file = tokio::fs::read_to_string(std::path::Path::new("./blog").join(md_path)).await.context_not_found("don't exist", "idk")?;
    render_template(&BlogTemplate { motd: motd()?, markdown: ferromark::to_html(&md_file) })
}

async fn blog_render_index() -> HtmlTemplate {
    blog_render(Path(PathBuf::from("index.md"))).await
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
    let config = init_config()?;

    let app = Router::new()
        .route("/", get(root))
        .route("/about", get(about))
        .route("/art", get(art))
        .route("/car", get(car))
        .route("/contact", get(contact))
        .route("/robots.txt", get(robots))
        .route("/i/am/very/smart", get(idiot))
        .route("/blog/", get(blog_render_index))
        .route("/blog", get(blog_render_index))
        .route("/blog/{*md_path}", get(blog_render))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/.well-known", ServeDir::new(".well-known"))
        .route("/.well-known/matrix/client", get(matrix_client))
        .route("/.well-known/matrix/server", get(matrix_server))
        // Set no-cache due to many dyanmic things on all parts of the website
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        // Both manual jail, and the layer internally set no-store
        .route("/jail", get(jail))
        .layer(host::HostCheckLayer::new(state.clone(), config))
        .with_state(state);

    let listen_addr = env::var("WEBSITE_BIND_ADDR").context("failed to read WEBSITE_BIND_ADDR")?;

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
