// Server URL + credentials → discover → pick a calendar. Used for CalDAV
// sources and CalDAV targets.

import { useState } from "react";
import { api, type CalDavConfig, type DiscoveredCalendar } from "./api";
import { ErrorBox } from "./components";
import { HelpLink } from "./help/HelpCenter";

export function CalDavPicker({ config, setConfig }: { config: CalDavConfig; setConfig: (c: CalDavConfig) => void }) {
  const [calendars, setCalendars] = useState<DiscoveredCalendar[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function discover() {
    setBusy(true);
    setError(null);
    try {
      const found = await api.post<DiscoveredCalendar[]>("/caldav/discover", {
        url: config.url ?? "",
        username: config.username ?? "",
        password: config.password ?? "",
      });
      setCalendars(found);
      if (found.length > 0 && !found.some((c) => c.url === config.calendar_url)) setConfig({ ...config, calendar_url: found[0].url });
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <label>
        CalDAV server
        <input value={config.url ?? ""} onChange={(e) => setConfig({ ...config, url: e.target.value })} required placeholder="https://caldav.icloud.com/  ·  https://cloud.example.com/remote.php/dav/" />
        <span className="hint">
          Most services need an app password rather than your normal one. Setup guide:{" "}
          <HelpLink id="mailbox">mailbox.org</HelpLink> · <HelpLink id="icloud">iCloud</HelpLink> ·{" "}
          <HelpLink id="fastmail">Fastmail</HelpLink> · <HelpLink id="nextcloud">Nextcloud</HelpLink>
        </span>
      </label>
      <div className="row">
        <label>
          Username
          <input value={config.username ?? ""} onChange={(e) => setConfig({ ...config, username: e.target.value })} required autoComplete="off" />
        </label>
        <label>
          Password
          <input type="password" value={config.password ?? ""} onChange={(e) => setConfig({ ...config, password: e.target.value })} required autoComplete="new-password" />
        </label>
      </div>
      <div className="row">
        <button type="button" className="secondary" disabled={busy} onClick={() => void discover()}>
          {busy ? "Looking…" : "Find calendars"}
        </button>
        {config.calendar_url && !calendars && <span className="muted small mono ellipsis">{config.calendar_url}</span>}
      </div>
      <ErrorBox error={error} />
      {calendars && (
        <label>
          Calendar
          <select value={config.calendar_url ?? ""} onChange={(e) => setConfig({ ...config, calendar_url: e.target.value })}>
            {calendars.map((c) => (
              <option key={c.url} value={c.url}>
                {c.name} — {c.url}
              </option>
            ))}
          </select>
        </label>
      )}
    </>
  );
}
