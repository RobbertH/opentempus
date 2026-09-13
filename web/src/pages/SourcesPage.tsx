import { useState, type FormEvent } from "react";
import { api, type Source } from "../api";
import { Badge, Empty, ErrorBox, Modal, Page, useAsync } from "../components";
import { fmtDateTime } from "../time";

const COLORS = ["#4f6df5", "#ff6600", "#16a34a", "#db2777", "#9333ea", "#0891b2", "#ca8a04", "#64748b"];

export default function SourcesPage() {
  const { data, error, reload } = useAsync(() => api.get<Source[]>("/sources"), []);
  const [editing, setEditing] = useState<Source | "new" | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  async function syncNow(s: Source) {
    setBusyId(s.id);
    setActionError(null);
    try {
      await api.post(`/sources/${s.id}/sync`);
      reload();
    } catch (e) {
      setActionError((e as Error).message);
    } finally {
      setBusyId(null);
    }
  }

  async function remove(s: Source) {
    if (!window.confirm(`Remove "${s.name}" and all its events?`)) return;
    try {
      await api.delete(`/sources/${s.id}`);
      reload();
    } catch (e) {
      setActionError((e as Error).message);
    }
  }

  return (
    <Page title="Calendars" actions={<button onClick={() => setEditing("new")}>Add calendar</button>}>
      <p className="muted">
        Calendars flow <em>in</em> here. Each one gets a category (work, personal, …) that your sharing rules can key on.
      </p>
      <ErrorBox error={error ?? actionError} />
      {data && data.length === 0 && <Empty>No calendars yet. Add the iCal/ICS address of any calendar to start syncing.</Empty>}
      <div className="list">
        {data?.map((s) => (
          <div className="card row-card" key={s.id}>
            <div className="swatch" style={{ background: s.color }} />
            <div className="grow">
              <div className="row">
                <b>{s.name}</b>
                <Badge>{s.category}</Badge>
                <Badge>{s.kind.replace("_", " ")}</Badge>
                {!s.enabled && <Badge tone="warn">paused</Badge>}
                <StatusBadge s={s} />
              </div>
              <div className="muted small mono ellipsis">{s.config.url}</div>
              <div className="muted small">
                Last sync {fmtDateTime(s.last_synced_at)} · every {Math.round(s.sync_interval_secs / 60)} min
                {s.last_sync_error && <span className="error-text"> · {s.last_sync_error}</span>}
              </div>
            </div>
            <div className="row">
              <button className="secondary small" disabled={busyId === s.id} onClick={() => void syncNow(s)}>
                {busyId === s.id ? "Syncing…" : "Sync now"}
              </button>
              <button className="secondary small" onClick={() => setEditing(s)}>
                Edit
              </button>
              <button className="danger small" onClick={() => void remove(s)}>
                Remove
              </button>
            </div>
          </div>
        ))}
      </div>
      {editing && (
        <SourceForm
          source={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            reload();
          }}
        />
      )}
    </Page>
  );
}

function StatusBadge({ s }: { s: Source }) {
  switch (s.last_sync_status) {
    case "ok":
      return <Badge tone="ok">synced</Badge>;
    case "error":
      return <Badge tone="error">error</Badge>;
    case "running":
      return <Badge tone="warn">syncing</Badge>;
    default:
      return <Badge>not synced yet</Badge>;
  }
}

function SourceForm({ source, onClose, onSaved }: { source: Source | null; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState(source?.name ?? "");
  const [url, setUrl] = useState(source?.config.url ?? "");
  const [username, setUsername] = useState(source?.config.username ?? "");
  const [password, setPassword] = useState(source?.config.password ?? "");
  const [category, setCategory] = useState(source?.category ?? "personal");
  const [color, setColor] = useState(source?.color ?? COLORS[0]);
  const [interval, setInterval] = useState(source ? Math.round(source.sync_interval_secs / 60) : 15);
  const [enabled, setEnabled] = useState(source?.enabled ?? true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    const config: Record<string, string> = { url };
    if (username) config.username = username;
    if (password) config.password = password;
    try {
      if (source) {
        await api.patch(`/sources/${source.id}`, { name, config, category, color, sync_interval_secs: interval * 60, enabled });
      } else {
        await api.post("/sources", { name, kind: "ics_url", config, category, color, sync_interval_secs: interval * 60 });
      }
      onSaved();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal title={source ? "Edit calendar" : "Add calendar"} onClose={onClose}>
      <form onSubmit={submit} className="form">
        <ErrorBox error={error} />
        <label>
          Name
          <input value={name} onChange={(e) => setName(e.target.value)} required placeholder="Work" />
        </label>
        <label>
          iCal / ICS address
          <input value={url} onChange={(e) => setUrl(e.target.value)} required placeholder="https://calendar.google.com/calendar/ical/…/basic.ics" />
          <span className="hint">
            Google: calendar settings → "Secret address in iCal format". Outlook: "Publish calendar" → ICS link. Apple/iCloud: share as public calendar (webcal://).
          </span>
        </label>
        <details>
          <summary className="muted small">Protected feed (basic auth)</summary>
          <div className="row">
            <label>
              Username
              <input value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="off" />
            </label>
            <label>
              Password
              <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="new-password" />
            </label>
          </div>
        </details>
        <div className="row">
          <label>
            Category
            <input value={category} onChange={(e) => setCategory(e.target.value)} list="categories" required />
            <datalist id="categories">
              <option value="work" />
              <option value="personal" />
              <option value="family" />
              <option value="sport" />
            </datalist>
          </label>
          <label>
            Sync every (minutes)
            <input type="number" min={1} max={1440} value={interval} onChange={(e) => setInterval(Number(e.target.value))} />
          </label>
        </div>
        <label>
          Color
          <div className="row">
            {COLORS.map((c) => (
              <button
                type="button"
                key={c}
                className={`swatch-btn ${c === color ? "selected" : ""}`}
                style={{ background: c }}
                onClick={() => setColor(c)}
                aria-label={c}
              />
            ))}
          </div>
        </label>
        {source && (
          <label className="inline">
            <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} /> Sync enabled
          </label>
        )}
        <div className="row end">
          <button type="button" className="secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" disabled={busy}>
            {busy ? "Saving & syncing…" : source ? "Save" : "Add & sync"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
