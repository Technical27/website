mod host;

use axum::{Router, extract::ConnectInfo, response::Html, routing::get};

use http::{HeaderValue, header};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

use std::net::SocketAddr;

use askama::Template;

use rand::prelude::*;

#[derive(Template)]
#[template(path = "index.html")]
struct RootTemplate<'a> {
    title: &'a str,
    source: &'a SocketAddr,
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate;

#[derive(Template)]
#[template(path = "art.html")]
struct ArtTemplate;

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
    "KDE is amazing"
];

fn motd() -> &'static str {
    MOTD.iter().choose(&mut rand::rng()).unwrap()
}

async fn root(connection: ConnectInfo<SocketAddr>) -> Html<String> {
    let root_template = RootTemplate {
        title: &motd(),
        source: &connection,
    };
    Html(root_template.render().unwrap())
}

async fn about() -> Html<String> {
    Html(AboutTemplate.render().unwrap())
}

async fn art() -> Html<String> {
    Html(ArtTemplate.render().unwrap())
}

async fn car() -> &'static str {
    "under construction, just use the back button"
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/about", get(about))
        .route("/art", get(art))
        .route("/car", get(car))
        .nest_service("/static", ServeDir::new("static"))
        .layer(host::HostCheckLayer::new())
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ));

    let listener = tokio::net::TcpListener::bind("[::]:3000").await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
