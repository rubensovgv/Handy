export interface StatisticsDay {
  date: string;
  words: number;
  sessions: number;
  seconds: number;
  measured_words: number;
}
export const dateKey = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
const ordinal = (key: string) => Date.parse(`${key}T00:00:00Z`) / 86400000;
export function summarize(days: StatisticsDay[], today: string) {
  let longest = 0,
    run = 0,
    last = -Infinity;
  for (const day of days
    .filter((d) => d.words > 0)
    .sort((a, b) => a.date.localeCompare(b.date))) {
    const n = ordinal(day.date);
    run = n === last + 1 ? run + 1 : 1;
    longest = Math.max(longest, run);
    last = n;
  }
  const totals = days.reduce(
    (a, d) => ({
      words: a.words + d.words,
      sessions: a.sessions + d.sessions,
      seconds: a.seconds + d.seconds,
      measured: a.measured + d.measured_words,
    }),
    { words: 0, sessions: 0, seconds: 0, measured: 0 },
  );
  return {
    ...totals,
    longest,
    current: ordinal(today) - last <= 1 ? run : 0,
    wpm:
      totals.seconds > 0
        ? Math.round((totals.measured * 60) / totals.seconds)
        : null,
  };
}
