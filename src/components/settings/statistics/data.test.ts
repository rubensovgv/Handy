import { strict as assert } from "node:assert";
import { summarize, type StatisticsDay } from "./data";
const day = (date: string, words = 60, seconds = 30): StatisticsDay => ({
  date,
  words,
  seconds,
  sessions: 1,
  measured_words: seconds ? words : 0,
});
assert.equal(summarize([], "2026-10-23").wpm, null);
const days = [
  day("2026-10-20"),
  day("2026-10-21"),
  day("2026-10-22", 120, 120),
  day("2026-10-01", 100, 0),
];
const stats = summarize(days, "2026-10-23");
assert.equal(stats.current, 3);
assert.equal(stats.longest, 3);
assert.equal(stats.words, 340);
assert.equal(stats.wpm, 80);
assert.equal(summarize(days, "2026-10-24").current, 0);
assert.equal(
  summarize(
    [day("2024-02-28"), day("2024-02-29"), day("2024-03-01")],
    "2024-03-01",
  ).current,
  3,
);
assert.equal(
  summarize([day("2025-12-31"), day("2026-01-01")], "2026-01-01").longest,
  2,
);
console.log(
  "Statistics: weighted WPM, missing audio, empty state, streak gaps, leap day and year boundary passed.",
);
