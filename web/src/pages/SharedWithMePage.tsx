import { useState } from "react";
import { api, type SharedEvent, type SharedWithMe } from "../api";
import { CopyButton, Empty, ErrorBox, Page, useAsync } from "../components";
import { fmtDateTime } from "../time";
import { VisibilitySummary } from "./SharesPage";

export default function SharedWithMePage() {
  const { data, error } = useAsync(() => api.get<SharedWithMe[]>("/shared-with-me"), []);
  const [open, setOpen] = useState<SharedWithMe | null>(null);

  return (
    <Page title="Shared with me">
      <p className="muted">Calendars your friends share with you. Subscribe to the feed URL from Google, Apple or Outlook to see them next to your own events.</p>
      <ErrorBox error={error} />
      {data && data.length === 0 && <Empty>Nobody is sharing a calendar with you yet.</Empty>}
      <div className="list">
        {data?.map((s) => (
          <div className="card" key={s.id}>
            <div className="row">
              <div className="grow">
                <b>{s.owner_display_name}</b> · {s.name}
                <div className="muted small">
                  <VisibilitySummary v={s.visibility} />
                </div>
              </div>
              <CopyButton text={s.feed_url} label="Copy feed URL" />
              <button className="secondary small" onClick={() => setOpen(open?.id === s.id ? null : s)}>
                {open?.id === s.id ? "Hide" : "Show events"}
              </button>
            </div>
            {open?.id === s.id && <EventList shareId={s.id} />}
          </div>
        ))}
      </div>
    </Page>
  );
}

function EventList({ shareId }: { shareId: string }) {
  const from = new Date();
  const to = new Date(from.getTime() + 14 * 86400_000);
  const { data, error } = useAsync(
    () => api.get<SharedEvent[]>(`/shared-with-me/${shareId}/events?from=${from.toISOString()}&to=${to.toISOString()}`),
    [shareId],
  );
  return (
    <div className="sub">
      <ErrorBox error={error} />
      {data && data.length === 0 && <div className="muted small">Nothing in the next 14 days.</div>}
      <SharedEventTable events={data ?? []} />
    </div>
  );
}

export function SharedEventTable({ events }: { events: SharedEvent[] }) {
  if (events.length === 0) return null;
  return (
    <table className="table">
      <thead>
        <tr>
          <th>When</th>
          <th>What</th>
          <th>Where</th>
          <th>Category</th>
          <th>RSVP</th>
        </tr>
      </thead>
      <tbody>
        {events.map((e) => (
          <tr key={e.id} className={e.busy ? "" : "muted"}>
            <td className="nowrap">
              {e.all_day ? new Date(e.start).toUTCString().slice(0, 16) + " (all day)" : `${fmtDateTime(e.start)} – ${new Date(e.end).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}`}
            </td>
            <td>
              {e.title ?? <span className="muted">{e.busy ? "Busy" : "Free"}</span>}
              {e.origin_calendar && <span className="muted small"> · {e.origin_calendar}</span>}
              {e.description && <div className="muted small pre">{e.description}</div>}
            </td>
            <td>{e.location ?? ""}</td>
            <td>{e.category ?? ""}</td>
            <td>{e.rsvp ?? ""}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
