import { useState, type FormEvent } from "react";
import { DEFAULT_FILTERS, api, type CalDavConfig, type Filters, type Preset, type SharedEvent, type Source, type Target, type Visibility } from "../api";
import { CalDavPicker } from "../CalDavPicker";
import { Badge, Empty, ErrorBox, Modal, Page, useAsync } from "../components";
import { HelpLink } from "../help/HelpCenter";
import { FiltersEditor, VisibilityEditor, filterWords, visibilityWords } from "../RuleEditor";
import { fmtDateTime } from "../time";
import { SharedEventTable } from "./SharedWithMePage";

export default function TargetsPage() {
  const targets = useAsync(() => api.get<Target[]>("/targets"), []);
  const sources = useAsync(() => api.get<Source[]>("/sources"), []);
  const presets = useAsync(() => api.get<Preset[]>("/shares/presets"), []);
  const [editing, setEditing] = useState<Target | "new" | null>(null);
  const [preview, setPreview] = useState<Target | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  async function pushNow(t: Target) {
    setBusyId(t.id);
    setErr(null);
    try {
      await api.post(`/targets/${t.id}/push`);
      targets.reload();
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setBusyId(null);
    }
  }
  async function remove(t: Target) {
    const clear = window.confirm(`Remove "${t.name}"?\n\nOK: also delete the ${t.mirrored_count} mirrored events from the remote calendar.\nCancel: keep everything.`);
    if (!clear) return;
    try {
      await api.delete(`/targets/${t.id}?clear_remote=true`);
      targets.reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }
  async function toggle(t: Target) {
    try {
      await api.patch(`/targets/${t.id}`, { enabled: !t.enabled });
      targets.reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }

  return (
    <Page title="Sync to calendars" actions={<button onClick={() => setEditing("new")}>Add sync target</button>}>
      <p className="muted">
        Write a filtered view of your calendars <em>into</em> another calendar, for example your personal blockers into your work calendar as "Busy". Mirrored events are tagged so they are never synced back.
      </p>
      <ErrorBox error={targets.error ?? err} />
      {targets.data && targets.data.length === 0 && (
        <Empty>
          No sync targets yet. Add one to push busy blocks into a CalDAV calendar. See the{" "}
          <HelpLink>setup guides</HelpLink> for mailbox.org, iCloud, Fastmail and Nextcloud. Google and Outlook cannot
          receive events yet.
        </Empty>
      )}
      <div className="list">
        {targets.data?.map((t) => (
          <div className="card" key={t.id}>
            <div className="row">
              <div className="grow">
                <div className="row">
                  <b>{t.name}</b>
                  <Badge>{t.kind}</Badge>
                  {!t.enabled && <Badge tone="warn">paused</Badge>}
                  <PushBadge t={t} />
                </div>
                <div className="muted small mono ellipsis">→ {t.config.calendar_url}</div>
                <div className="muted small">
                  Shows: {visibilityWords(t.visibility).join(", ")} · {filterWords(t.filters, sources.data ?? []).join(" · ")}
                </div>
                <div className="muted small">
                  {t.mirrored_count} mirrored · last push {fmtDateTime(t.last_pushed_at)}
                  {t.last_push_error && <span className="error-text"> · {t.last_push_error}</span>}
                </div>
              </div>
              <button className="secondary small" disabled={busyId === t.id} onClick={() => void pushNow(t)}>
                {busyId === t.id ? "Pushing…" : "Push now"}
              </button>
              <button className="secondary small" onClick={() => setPreview(t)}>
                Preview
              </button>
              <button className="secondary small" onClick={() => setEditing(t)}>
                Edit
              </button>
              <button className="secondary small" onClick={() => void toggle(t)}>
                {t.enabled ? "Pause" : "Resume"}
              </button>
              <button className="danger small" onClick={() => void remove(t)}>
                Remove
              </button>
            </div>
          </div>
        ))}
      </div>
      {editing && sources.data && presets.data && (
        <TargetForm
          target={editing === "new" ? null : editing}
          sources={sources.data}
          presets={presets.data}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            targets.reload();
          }}
        />
      )}
      {preview && <PreviewModal target={preview} onClose={() => setPreview(null)} />}
    </Page>
  );
}

function PushBadge({ t }: { t: Target }) {
  switch (t.last_push_status) {
    case "ok":
      return <Badge tone="ok">in sync</Badge>;
    case "error":
      return <Badge tone="error">error</Badge>;
    case "running":
      return <Badge tone="warn">pushing</Badge>;
    default:
      return <Badge>not pushed yet</Badge>;
  }
}

function TargetForm({ target, sources, presets, onClose, onSaved }: { target: Target | null; sources: Source[]; presets: Preset[]; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState(target?.name ?? "");
  const [config, setConfig] = useState<CalDavConfig>(target?.config ?? {});
  const [vis, setVis] = useState<Visibility>(target?.visibility ?? presets[0].visibility);
  const [filters, setFilters] = useState<Filters>(target?.filters ?? { ...DEFAULT_FILTERS, horizon_past_days: 0, horizon_future_days: 60 });
  const [placeholder, setPlaceholder] = useState(target?.placeholder_title ?? "Busy");
  const [interval, setInterval] = useState(target ? Math.round(target.push_interval_secs / 60) : 15);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const body = { name, config, visibility: vis, filters, placeholder_title: placeholder, push_interval_secs: interval * 60 };
      if (target) await api.patch(`/targets/${target.id}`, body);
      else await api.post("/targets", { ...body, kind: "caldav" });
      onSaved();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal title={target ? "Edit sync target" : "Add sync target"} onClose={onClose}>
      <form onSubmit={submit} className="form">
        <ErrorBox error={error} />
        <label>
          Name
          <input value={name} onChange={(e) => setName(e.target.value)} required placeholder="Personal blockers → work" />
        </label>
        <fieldset>
          <legend>Destination calendar (CalDAV)</legend>
          <CalDavPicker config={config} setConfig={setConfig} />
        </fieldset>
        <VisibilityEditor vis={vis} setVis={setVis} presets={presets} />
        <FiltersEditor filters={filters} setFilters={setFilters} sources={sources} />
        <div className="row">
          <label>
            Title when hidden
            <input value={placeholder} onChange={(e) => setPlaceholder(e.target.value)} placeholder="Busy" />
          </label>
          <label>
            Push every (minutes)
            <input type="number" min={1} max={1440} value={interval} onChange={(e) => setInterval(Number(e.target.value))} />
            <span className="hint">Pushes also run right after any of your calendars changes.</span>
          </label>
        </div>
        <div className="row end">
          <button type="button" className="secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" disabled={busy || !config.calendar_url}>
            {busy ? "Saving & pushing…" : target ? "Save" : "Create & push"}
          </button>
        </div>
      </form>
    </Modal>
  );
}

function PreviewModal({ target, onClose }: { target: Target; onClose: () => void }) {
  const from = new Date();
  const to = new Date(from.getTime() + 14 * 86400_000);
  const { data, error, loading } = useAsync(() => api.get<SharedEvent[]>(`/targets/${target.id}/preview?from=${from.toISOString()}&to=${to.toISOString()}`), [target.id]);
  return (
    <Modal title={`What "${target.name}" writes (next 14 days)`} onClose={onClose}>
      <ErrorBox error={error} />
      {loading && <div className="muted">Loading…</div>}
      {data && data.length === 0 && <Empty>Nothing would be written in the next 14 days.</Empty>}
      <SharedEventTable events={data ?? []} />
    </Modal>
  );
}
