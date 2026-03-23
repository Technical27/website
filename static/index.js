"use strict";

const local = luxon.DateTime.now();

const list = document.getElementById("fun-list");
const messages = [];

// Indonesia and the Philippines both have distintive timezones, but for some stupid reason,
// windows groups all the timezones for these regions with many others despite not being the same at all.
// just because this is a really really stupid funny joke, just use UTC offsets since both places do not observe DST.

// UTC+7
if (local.offset === 420) {
  messages.push("hi bramble");
// UTC+8
} else if (local.offset === 480) {
  messages.push("website programming isn't easy ok");
// Check if time is the same as New York, easiest way to account for DST differences
} else {
  const est = local.setZone("America/New_York");
  if (est.offset === local.offset) {
    if (est.isInDST) {
      messages.push("Eastern Standard Time gang, daylight edition");
    } else {
      messages.push("Eastern Standard Time gang");
    }
  }
}

// I am far too lazy to check every single Australia timezone, just hope this is good enough unless windows is stupid again
if (local.zone.ianaName.startsWith("Australia")) {
  messages.push("Hi to you all in Australia!");
}

if (local.offset % 60 !== 0) {
  messages.push("Thank you for being an example of why UTC offsets are a bad way to measure time. Your current UTC offset is " + local.toFormat("ZZ"))
}

if (local.hour > 2 && local.hour < 6) {
  messages.push("<li>you should go to sleep its " + local.toLocaleString(luxon.DateTime.TIME_SIMPLE) + "</li>");
}

list.innerHTML += messages.map(x => `<li>${x}</li>`).join('\n')
