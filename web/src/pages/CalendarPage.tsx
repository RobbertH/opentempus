import { useMemo, useState } from "react";
import { api, type MyEvent } from "../api";
import { Empty, ErrorBox, Page, useAsync } from "../components";
import { addDays, allDayDate, fmtDay, fmtTime, minutesIntoDay, sameDay, startOfWeek } from "../time";

const HOUR_PX = 44;

export default function CalendarPage() {
  const [weekStart, setWeekStart] = useState(() => startOfWeek(new Date()));
  const weekEnd = useMemo(() => addDays(weekStart, 7), [weekStart]);
  const { data, error, loading } = useAsync(
    () => api.get<MyEvent[]>(`/events?from=${weekStart.toISOString()}&to=${weekEnd.toISOString()}`),
    [weekStart],
  );
  const days = useMemo(() => Array.from({ length: 7 }, (_, i) => addDays(weekStart, i)), [weekStart]);
  const today = new Date();

  return (
    <Page
      title="Calendar"
      actions={
        <div className="row">
          <button className="secondary" onClick={() => setWeekStart(addDays(weekStart, -7))}>
            ‹
          </button>
          <button className="secondary" onClick={() => setWeekStart(startOfWeek(new Date()))}>
            Today
          </button>
          <button className="secondary" onClick={() => setWeekStart(addDays(weekStart, 7))}>
            ›
          </button>
          <span className="muted">
            {fmtDay(weekStart)} – {fmtDay(addDays(weekStart, 6))}
          </span>
        </div>
      }
    >
      <ErrorBox error={error} />
      {!loading && data && data.length === 0 && (
        <Empty>
          Nothing on this week. Add a calendar under <b>Calendars</b> to get started.
        </Empty>
      )}
      <div className="week">
        <div className="week-head">
          <div className="gutter" />
          {days.map((d) => (
            <div key={d.toISOString()} className={`day-head ${sameDay(d, today) ? "today" : ""}`}>
              {fmtDay(d)}
            </div>
          ))}
        </div>
        <div className="week-allday">
          <div className="gutter small muted">all day</div>
          {days.map((d) => (
            <div key={d.toISOString()} className="day-allday">
              {(data ?? [])
                .filter((e) => e.all_day && allDayDate(e.start) <= d && allDayDate(e.end) > d)
                .map((e) => (
                  <EventChip key={e.id} ev={e} />
                ))}
            </div>
          ))}
        </div>
        <div className="week-body" style={{ height: HOUR_PX * 24 }}>
          <div className="gutter">
            {Array.from({ length: 23 }, (_, i) => i + 1).map((h) => (
              <div key={h} className="hour-label" style={{ top: h * HOUR_PX }}>
                {String(h).padStart(2, "0")}:00
              </div>
            ))}
          </div>
          {days.map((d) => (
            <div key={d.toISOString()} className={`day-col ${sameDay(d, today) ? "today" : ""}`}>
              {Array.from({ length: 24 }, (_, h) => (
                <div key={h} className="hour-line" style={{ top: h * HOUR_PX }} />
              ))}
              {(data ?? [])
                .filter((e) => !e.all_day)
                .flatMap((e) => segmentsForDay(e, d))
                .map(({ ev, top, height, key }) => (
                  <div
                    key={key}
                    className={`event ${ev.transparency === "transparent" ? "free" : ""} ${ev.rsvp === "declined" ? "declined" : ""} ${ev.rsvp === "tentative" || ev.rsvp === "needs_action" || ev.status === "tentative" ? "tentative" : ""}`}
                    style={{ top, height: Math.max(height, 18), borderLeftColor: ev.color, background: `${ev.color}22` }}
                    title={`${ev.title ?? "(untitled)"}\n${fmtTime(ev.start)} – ${fmtTime(ev.end)}\n${ev.source_name} · ${ev.category} · ${ev.rsvp}${ev.location ? `\n${ev.location}` : ""}`}
                  >
                    <div className="event-title">{ev.title ?? "(untitled)"}</div>
                    <div className="event-time">
                      {fmtTime(ev.start)} – {fmtTime(ev.end)}
                    </div>
                  </div>
                ))}
            </div>
          ))}
        </div>
      </div>
    </Page>
  );
}

function segmentsForDay(ev: MyEvent, day: Date): { ev: MyEvent; top: number; height: number; key: string }[] {
  const dayStart = new Date(day);
  dayStart.setHours(0, 0, 0, 0);
  const dayEnd = addDays(dayStart, 1);
  const s = new Date(ev.start);
  const e = new Date(ev.end);
  if (e <= dayStart || s >= dayEnd) return [];
  const from = s < dayStart ? 0 : minutesIntoDay(s);
  const to = e > dayEnd ? 24 * 60 : minutesIntoDay(e);
  return [{ ev, top: (from / 60) * HOUR_PX, height: ((to - from) / 60) * HOUR_PX, key: `${ev.id}-${day.getDate()}` }];
}

function EventChip({ ev }: { ev: MyEvent }) {
  return (
    <div className="chip" style={{ background: `${ev.color}33`, borderColor: ev.color }} title={`${ev.source_name} · ${ev.category}`}>
      {ev.title ?? "(untitled)"}
    </div>
  );
}
