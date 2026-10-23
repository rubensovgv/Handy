import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import {
  ChevronLeft,
  ChevronRight,
  BarChart3,
  Flame,
  Gauge,
  Mic,
} from "lucide-react";
import { events } from "@/bindings";
import { dateKey, summarize, type StatisticsDay } from "./data";

export function StatisticsSettings() {
  const { t, i18n } = useTranslation();
  const [days, setDays] = useState<StatisticsDay[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState(false);
  const [today, setToday] = useState(() => dateKey(new Date()));
  const [year, setYear] = useState(() => new Date().getFullYear());
  const [refresh, setRefresh] = useState(0);
  useEffect(() => {
    let active = true;
    let generation = 0;
    const load = async () => {
      const request = ++generation;
      try {
        const data = await invoke<StatisticsDay[]>("get_dictation_statistics");
        if (active && request === generation) {
          setDays(data);
          setLoaded(true);
          setError(false);
          setToday(dateKey(new Date()));
        }
      } catch {
        if (active && request === generation) setError(true);
      }
    };
    void load();
    const listener = events.historyUpdatePayload.listen(() => void load());
    window.addEventListener("focus", load);
    const timer = window.setInterval(
      () => setToday(dateKey(new Date())),
      60000,
    );
    return () => {
      active = false;
      void listener.then((unlisten) => unlisten());
      window.removeEventListener("focus", load);
      window.clearInterval(timer);
    };
  }, [refresh]);
  const stats = summarize(days, today);
  const number = (n: number) => n.toLocaleString(i18n.language);
  const lookup = new Map(days.map((day) => [day.date, day]));
  const currentMonth = today.slice(0, 7);
  const previous = new Date(
    Number(today.slice(0, 4)),
    Number(today.slice(5, 7)) - 2,
    1,
  );
  const previousMonth = dateKey(previous).slice(0, 7);
  const monthWords = days
    .filter((d) => d.date.startsWith(currentMonth))
    .reduce((sum, d) => sum + d.words, 0);
  const previousWords = days
    .filter((d) => d.date.startsWith(previousMonth))
    .reduce((sum, d) => sum + d.words, 0);
  const start = new Date(year, 0, 1);
  start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  const end = new Date(year, 11, 31);
  const cells: Date[] = [];
  for (
    const date = new Date(start);
    date <= end || cells.length % 7 !== 0;
    date.setDate(date.getDate() + 1)
  )
    cells.push(new Date(date));
  const max = Math.max(
    1,
    ...days.filter((d) => d.date.startsWith(String(year))).map((d) => d.words),
  );
  if (error)
    return (
      <div className="p-6 space-y-3">
        <p role="alert">{t("statistics.error")}</p>
        <button
          className="text-logo-primary underline"
          onClick={() => setRefresh((n) => n + 1)}
        >
          {t("statistics.retry")}
        </button>
      </div>
    );
  if (!loaded)
    return (
      <p className="p-6" role="status">
        {t("statistics.loading")}
      </p>
    );
  const card = "rounded-xl border border-mid-gray/20 p-5 bg-mid-gray/5";
  return (
    <div className="max-w-3xl w-full mx-auto space-y-5">
      <header>
        <h1 className="text-xl font-semibold">{t("sidebar.statistics")}</h1>
        <p className="text-sm text-text/60 mt-1">{t("statistics.subtitle")}</p>
      </header>
      <section className={card}>
        <div className="flex items-center justify-between gap-4">
          <BarChart3 className="text-logo-primary" size={22} />
          <span className="text-xs text-text/60">
            {t("statistics.month", {
              count: monthWords,
              formatted: number(monthWords),
            })}
          </span>
        </div>
        <p className="text-4xl font-semibold tracking-tight mt-3">
          {number(stats.words)}
        </p>
        <h2 className="text-sm text-text/60 mt-1">{t("statistics.total")}</h2>
        <div className="mt-4 pt-3 border-t border-mid-gray/20 text-sm text-text/70">
          {previousWords > 0
            ? t("statistics.comparison", {
                value: `${monthWords >= previousWords ? "+" : ""}${Math.round((monthWords / previousWords - 1) * 100)}`,
              })
            : t("statistics.noComparison")}
        </div>
      </section>
      <div className="grid grid-cols-2 gap-4">
        <section className={card}>
          <Gauge size={20} className="text-logo-primary" />
          <p className="text-3xl font-semibold mt-3">
            {stats.wpm === null ? "—" : number(stats.wpm)}
          </p>
          <h2 className="text-sm text-text/60">{t("statistics.wpm")}</h2>
          <p className="text-xs text-text/50 mt-2">{t("statistics.wpmNote")}</p>
        </section>
        <section className={card}>
          <Mic size={20} className="text-logo-primary" />
          <p className="text-3xl font-semibold mt-3">
            {number(stats.sessions)}
          </p>
          <h2 className="text-sm text-text/60">{t("statistics.sessions")}</h2>
          <p className="text-xs text-text/50 mt-2">
            {t("statistics.minutes", {
              value: number(Math.round(stats.seconds / 60)),
            })}
          </p>
        </section>
      </div>
      <section className={card}>
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-xl font-semibold flex items-center gap-2">
              <Flame size={20} className="text-logo-primary" />
              {t("statistics.streak", { count: stats.current })}
            </h2>
            <p className="text-xs text-text/60 mt-1">
              {t("statistics.longest", { count: stats.longest })}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              className="p-1 rounded hover:bg-mid-gray/20"
              aria-label={t("statistics.previousYear")}
              onClick={() => setYear((y) => y - 1)}
            >
              <ChevronLeft size={18} />
            </button>
            <span className="text-sm tabular-nums">{year}</span>
            <button
              className="p-1 rounded hover:bg-mid-gray/20 disabled:opacity-30"
              disabled={year >= Number(today.slice(0, 4))}
              aria-label={t("statistics.nextYear")}
              onClick={() => setYear((y) => y + 1)}
            >
              <ChevronRight size={18} />
            </button>
          </div>
        </div>
        <div className="overflow-x-auto mt-5 pb-2">
          <div style={{ minWidth: 490 }}>
            <div className="flex justify-between text-xs text-text/50 mb-2">
              {Array.from({ length: 12 }, (_, month) => (
                <span key={month}>
                  {new Date(year, month, 1).toLocaleDateString(i18n.language, {
                    month: "short",
                  })}
                </span>
              ))}
            </div>
            <div
              className="grid gap-[3px]"
              style={{
                gridTemplateRows: "repeat(7, 10px)",
                gridAutoFlow: "column",
                gridAutoColumns: "1fr",
              }}
            >
              {cells.map((date) => {
                const key = dateKey(date),
                  words = lookup.get(key)?.words ?? 0,
                  outside = date.getFullYear() !== year || key > today;
                return (
                  <div
                    key={key}
                    role="img"
                    aria-label={t("statistics.day", {
                      date: date.toLocaleDateString(i18n.language),
                      value: number(words),
                    })}
                    title={t("statistics.day", {
                      date: date.toLocaleDateString(i18n.language),
                      value: number(words),
                    })}
                    className="rounded-[2px]"
                    style={{
                      visibility: outside ? "hidden" : undefined,
                      background: words
                        ? `color-mix(in srgb, var(--color-logo-primary) ${30 + 70 * Math.sqrt(words / max)}%, var(--color-background))`
                        : "color-mix(in srgb, var(--color-mid-gray) 15%, transparent)",
                    }}
                  />
                );
              })}
            </div>
          </div>
        </div>
        <div className="flex justify-end items-center gap-1.5 text-xs text-text/50 mt-2">
          <span>{t("statistics.less")}</span>
          {[15, 35, 65, 100].map((opacity) => (
            <span
              key={opacity}
              className="w-2.5 h-2.5 rounded-[2px] bg-logo-primary"
              style={{ opacity: opacity / 100 }}
            />
          ))}
          <span>{t("statistics.more")}</span>
        </div>
        {stats.words === 0 && (
          <p className="text-sm text-text/60 mt-4">{t("statistics.empty")}</p>
        )}
      </section>
      <p className="text-xs text-text/50 leading-relaxed">
        {t("statistics.storage")}
      </p>
    </div>
  );
}
