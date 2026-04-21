"use strict";

const headingText = "Aamaruvi";
const subheadingText = "hi. and welcome.";
const paraText = "i can't be here for too long.\nfind me elsewhere to truly know me.\nthis interface may\rchange and reconfigure at only my will.\nthe hyperlinks are all\ri can give you right now.\nmore to come when i am able.";

const h1 = document.getElementById("title");
const h2 = document.getElementById("subtitle");
const para = document.getElementById("para");

const links = [["GOTO", document.getElementById("link1")], ["JMP", document.getElementById("link2")],
["window.location.href =", document.getElementById("link3")], ["wget", document.getElementById("link4")], ["curl", document.getElementById("link5")]];

const genChar = () => {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890*&^$#@!()'\"[]{}-_=+;;,.<>/?`~\\|";
  return chars.charAt(Math.random() * chars.length);
}

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
        //el.innerText = el.getAttribute("href");
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

/* setTimeout(() => {
  for (const el of document.getElementsByTagName("a")) {
    el.innerText = el.getAttribute("href");
  }
}, 60); */

decLoop();
