"use strict";

const local = luxon.DateTime.now();

const list = document.getElementById("fun-list");

if (local.offset === 420) {
  list.innerHTML += "<li>hi bramble</li>";
} else if (local.zone.ianaName.startsWith("Australia")) {
  list.innerHTML += "<li>Hi to you all in Australia!</li>";
} else if (local.offset === 480) {
  list.innerHTML += "<li>website programming isn't easy ok</li>";
} else {
  const est = local.setZone("America/New_York");
  if (est.offset === local.offset) {
    if (est.isInDST) {
      list.innerHTML += "<li>Eastern Standard Time gang, daylight edition</li>"
    } else {
      list.innerHTML += "<li>Eastern Standard Time gang</li>"
    }
  }
}

if (local.hour > 2 && local.hour < 6) {
  list.innerHTML += "<li>you should go to sleep its " + local.toLocaleString(luxon.DateTime.TIME_SIMPLE) + "</li>";
}
