"use strict";

const headingText = "Aamaruvi";
const subheadingText = "hi. and welcome.";
const paraText = "i can't be here for too long.\nfind me elsewhere to truly know me.\nthis interface may\rchange and reconfigure at only my will.\nthe hyperlinks are all\ri can give you right now.\nmore to come when i am able.";

const h1 = document.getElementById("title");
const h2 = document.getElementById("subtitle");
const para = document.getElementById("para");

const links = [["GOTO", document.getElementById("link1")], ["JMP", document.getElementById("link2")],
["window.location.href =", document.getElementById("link3")], ["wget", document.getElementById("link4")], ["curl", document.getElementById("link5")]];

const list = document.getElementById("fun-list");
const local = luxon.DateTime.now();

// Indonesia and the Philippines both have distintive timezones, but for some stupid reason,
// windows groups all the timezones for these regions with many others despite not being the same at all.
// just because this is a really really stupid funny joke, just use UTC offsets since both places do not observe DST.

// UTC+7
if (local.offset === 420) {
  messages.push("USER=bramble");
  // UTC+8
} else if (local.offset === 480) {
  messages.push("USER=quote");
  // Check if time is the same as New York, easiest way to account for DST differences
} else {
  const est = local.setZone("America/New_York");
  if (est.offset === local.offset) {
    if (est.isInDST) {
      messages.push("TZ=EDT");
    } else {
      messages.push("TZ=EST");
    }
  }
}

// I am far too lazy to check every single Australia timezone, just hope this is good enough unless windows is stupid again
if (local.zone.ianaName.startsWith("Australia")) {
  messages.push("TZ+=AUS");
}

if (local.offset % 60 !== 0) {
  messages.push("UTC=" + local.toFormat("ZZ"))
}

if (local.hour > 2 && local.hour < 6) {
  messages.push("SLP");
}

const genChar = () => {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890*&^$#@!()'\"[]{}-_=+;;,.<>/?`~\\|";
  return chars.charAt(Math.random() * chars.length);
}

list.innerText = 'STATUS: ' + messages.join(', ')

const decText = (el, text, idx) => {
  let b = [];
  for (let i = 0; i < text.length; i++) {
    if (text.charAt(i) == '\r') {
      if (window.innerWidth < 520) {
        b.push('\n');
      } else {
        b.push(' ');
      }
    } else if (text.charAt(i) == '\n') {
      b.push('\n');
    } else if (i < idx) {
      b.push(text.charAt(i));
    } else {
      b.push(genChar());
    }
  }

  el.innerText = b.join('');
}

let dt = 1;
let done = false;
const decLoop = () => {
  if (dt < paraText.length + 32) {
    setTimeout(() => {
      decText(h1, headingText, dt);
      decText(h2, subheadingText, dt);
      decText(para, paraText, dt);
      for (const [text, el] of links) {
        decText(el, text, dt);
      }
      for (const el of document.getElementsByTagName("a")) {
        decText(el, el.getAttribute("href"), dt);
      }
      if (dt > paraText.length / 2) {
        dt += 32;
      } else if (dt > subheadingText.length) {
        dt += 16;
      } else if (dt > headingText.length) {
        dt += 8;
      } else {
        dt += 1;
      }
      decLoop();
    }, 60);
  } else {
    done = true;
  }
};

window.addEventListener("resize", () => {
  if (done) decText(para, paraText, dt);
})

decLoop();
