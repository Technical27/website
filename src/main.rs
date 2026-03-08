mod host;

use axum::{Router, extract::ConnectInfo, response::Html, routing::get};

use axum::Json;
use http::{HeaderValue, header};
use serde_json::{Value, json};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

use std::net::SocketAddr;

use askama::Template;

use rand::prelude::*;

use anyhow::{Result, anyhow};
use axum_anyhow::ApiResult;

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

async fn matrix_client() -> Json<Value> {
    Json(json!({ "m.homeserver": { "base_url": "https://matrix.aamaruvi.com" } }))
}

async fn matrix_server() -> Json<Value> {
    Json(json!({ "m.server": "matrix.aamaruvi.com:443" }))
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new()
        .route("/", get(root))
        .route("/about", get(about))
        .route("/art", get(art))
        .route("/car", get(car))
        .route("/.well-known/matrix/client", get(matrix_client))
        .route("/.well-known/matrix/server", get(matrix_server))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/.well-known", ServeDir::new(".well-known"))
        // Set no-cache due to many dyanmic things on all parts of the website
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        // Internally sets no-store
        .layer(host::HostCheckLayer::new());

    // TODO: don't hardcode this
    let listener = tokio::net::TcpListener::bind("[::]:14367").await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
