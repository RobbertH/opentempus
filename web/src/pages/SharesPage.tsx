import { useEffect, useMemo, useState, type FormEvent } from "react";
import { ALL_RSVP, DEFAULT_FILTERS, api, type Filters, type Friend, type Preset, type Rsvp, type Share, type SharedEvent, type Source, type Visibility } from "../api";
import { Badge, CopyButton, Empty, ErrorBox, Modal, Page, useAsync } from "../components";
import { SharedEventTable } from "./SharedWithMePage";

export default function SharesPage() {
  const shares = useAsync(() => api.get<Share[]>("/shares"), []);
  const sources = useAsync(() => api.get<Source[]>("/sources"), []);
  const friends = useAsync(() => api.get<Friend[]>("/friends"), []);
  const presets = useAsync(() => api.get<Preset[]>("/shares/presets"), []);
  const [editing, setEditing] = useState<Share | "new" | null>(null);
  const [preview, setPreview] = useState<Share | null>(null);
  const [err, setErr] = useState<string | null>(null);

  async function remove(s: Share) {
    if (!window.confirm(`Delete share "${s.name}"? Anyone using its link loses access immediately.`)) return;
    try {
      await api.delete(`/shares/${s.id}`);
      shares.reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }
  async function rotate(s: Share) {
    if (!window.confirm("Generate a new secret link? The old one stops working.")) return;
    try {
      await api.post(`/shares/${s.id}/rotate-token`);
      shares.reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }
  async function toggle(s: Share) {
    try {
      await api.patch(`/shares/${s.id}`, { enabled: !s.enabled });
      shares.reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }

  const acceptedFriends = friends.data?.filter((f) => f.status === "accepted") ?? [];

  return (
    <Page title="Sharing" actions={<button onClick={() => setEditing("new")}>New share</button>}>
      <p className="muted">
        A share is a rule: <em>which</em> events go out and <em>what</em> the audience may see of them. Share with a friend, or create a secret link for a calendar app or an AI agent.
      </p>
      <ErrorBox error={shares.error ?? err} />
      {shares.data && shares.data.length === 0 && <Empty>No shares yet. Create one to give a friend or an agent a filtered view of your calendars.</Empty>}
      <div className="list">
        {shares.data?.map((s) => (
          <div className="card" key={s.id}>
            <div className="row">
              <div className="grow">
                <div className="row">
                  <b>{s.name}</b>
                  {s.audience_kind === "friend" ? <Badge tone="ok">friend · {s.audience_display_name}</Badge> : <Badge>secret link</Badge>}
                  {!s.enabled && <Badge tone="warn">disabled</Badge>}
                </div>
                <div className="muted small">
                  <VisibilitySummary v={s.visibility} /> · <FilterSummary f={s.filters} sources={sources.data ?? []} />
                </div>
              </div>
              <button className="secondary small" onClick={() => setPreview(s)}>
                Preview
              </button>
              <button className="secondary small" onClick={() => setEditing(s)}>
                Edit
              </button>
              <button className="secondary small" onClick={() => void toggle(s)}>
                {s.enabled ? "Disable" : "Enable"}
              </button>
              <button className="danger small" onClick={() => void remove(s)}>
                Delete
              </button>
            </div>
            <div className="links">
              <div className="row">
                <span className="muted small nowrap">Feed (subscribe in Google/Apple/Outlook)</span>
                <code className="ellipsis grow">{s.feed_url}</code>
                <CopyButton text={s.feed_url} />
              </div>
              <div className="row">
                <span className="muted small nowrap">API (for agents)</span>
                <code className="ellipsis grow">{s.api_url}</code>
                <CopyButton text={s.api_url} />
                <button className="secondary small" onClick={() => void rotate(s)}>
                  Rotate secret
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
      {editing && sources.data && presets.data && (
        <ShareForm
          share={editing === "new" ? null : editing}
          sources={sources.data}
          friends={acceptedFriends}
          presets={presets.data}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            shares.reload();
          }}
        />
      )}
      {preview && <PreviewModal share={preview} onClose={() => setPreview(null)} />}
    </Page>
  );
}

export function VisibilitySummary({ v }: { v: Visibility }) {
  const parts: string[] = ["busy blocks"];
  if (v.category) parts.push("category");
  if (v.origin_calendar) parts.push("calendar name");
  if (v.title) parts.push("titles");
  if (v.location) parts.push("locations");
  if (v.rsvp) parts.push("RSVP");
  if (v.free_busy) parts.push("free/busy");
  if (v.description) parts.push("descriptions");
  return <>Shows: {parts.join(", ")}</>;
}

function FilterSummary({ f, sources }: { f: Filters; sources: Source[] }) {
  const parts: string[] = [];
  if (f.source_ids) parts.push(`calendars: ${f.source_ids.map((id) => sources.find((s) => s.id === id)?.name ?? "?").join(", ")}`);
  else parts.push("all calendars");
  if (f.categories) parts.push(`categories: ${f.categories.join(", ")}`);
  if (f.rsvp.includes("declined")) parts.push("incl. declined");
  if (f.include_free) parts.push("incl. free time");
  if (!f.include_all_day) parts.push("no all-day");
  parts.push(`${f.horizon_past_days}d back / ${f.horizon_future_days}d ahead`);
  return <>{parts.join(" · ")}</>;
}

const VIS_FIELDS: { key: keyof Visibility; label: string; hint: string }[] = [
  { key: "category", label: "Category", hint: "work / personal / … of the origin calendar" },
  { key: "origin_calendar", label: "Calendar name", hint: "which of your calendars the event is from" },
  { key: "title", label: "Title", hint: "the event's summary" },
  { key: "location", label: "Location", hint: "" },
  { key: "rsvp", label: "RSVP", hint: "whether you accepted, tentatively accepted, or have not replied" },
  { key: "free_busy", label: "Free/busy", hint: "distinguish events that do not actually block you" },
  { key: "description", label: "Description", hint: "full notes; often contains meeting links" },
];

function ShareForm({
  share,
  sources,
  friends,
  presets,
  onClose,
  onSaved,
}: {
  share: Share | null;
  sources: Source[];
  friends: Friend[];
  presets: Preset[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState(share?.name ?? "");
  const [kind, setKind] = useState<"friend" | "link">(share?.audience_kind ?? (friends.length > 0 ? "friend" : "link"));
  const [friendId, setFriendId] = useState(share?.audience_user_id ?? friends[0]?.user_id ?? "");
  const [vis, setVis] = useState<Visibility>(share?.visibility ?? presets[0].visibility);
  const [filters, setFilters] = useState<Filters>(share?.filters ?? DEFAULT_FILTERS);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const categories = useMemo(() => Array.from(new Set(sources.map((s) => s.category))).sort(), [sources]);
  const activePreset = presets.find((p) => JSON.stringify(p.visibility) === JSON.stringify(vis))?.id ?? "custom";

  useEffect(() => {
    if (!name && kind === "friend") {
      const f = friends.find((x) => x.user_id === friendId);
      if (f) setName(`For ${f.display_name}`);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [friendId, kind]);

  function toggleSource(id: string) {
    const current = filters.source_ids ?? sources.map((s) => s.id);
    const next = current.includes(id) ? current.filter((x) => x !== id) : [...current, id];
    setFilters({ ...filters, source_ids: next.length === sources.length ? null : next });
  }
  function toggleCategory(c: string) {
    const current = filters.categories ?? categories;
    const next = current.includes(c) ? current.filter((x) => x !== c) : [...current, c];
    setFilters({ ...filters, categories: next.length === categories.length ? null : next });
  }
  function toggleRsvp(r: Rsvp) {
    const next = filters.rsvp.includes(r) ? filters.rsvp.filter((x) => x !== r) : [...filters.rsvp, r];
    setFilters({ ...filters, rsvp: next });
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      if (share) {
        await api.patch(`/shares/${share.id}`, { name, visibility: vis, filters });
      } else {
        await api.post("/shares", {
          name,
          audience_kind: kind,
          audience_user_id: kind === "friend" ? friendId : null,
          visibility: vis,
          filters,
        });
      }
      onSaved();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal title={share ? "Edit share" : "New share"} onClose={onClose}>
      <form onSubmit={submit} className="form">
        <ErrorBox error={error} />
        {!share && (
          <div className="row">
            <label className="inline">
              <input type="radio" checked={kind === "friend"} onChange={() => setKind("friend")} disabled={friends.length === 0} /> Share with a friend
            </label>
            <label className="inline">
              <input type="radio" checked={kind === "link"} onChange={() => setKind("link")} /> Secret link (calendar app, AI agent)
            </label>
          </div>
        )}
        {!share && kind === "friend" && (
          <label>
            Friend
            <select value={friendId} onChange={(e) => setFriendId(e.target.value)}>
              {friends.map((f) => (
                <option key={f.user_id} value={f.user_id}>
                  {f.display_name} ({f.email})
                </option>
              ))}
            </select>
          </label>
        )}
        <label>
          Name
          <input value={name} onChange={(e) => setName(e.target.value)} required placeholder={kind === "link" ? "Padel agent" : "For Bob"} />
        </label>

        <fieldset>
          <legend>What they may see</legend>
          <div className="row wrap">
            {presets.map((p) => (
              <button type="button" key={p.id} className={`chip-btn ${activePreset === p.id ? "selected" : ""}`} onClick={() => setVis(p.visibility)} title={p.description}>
                {p.label}
              </button>
            ))}
            {activePreset === "custom" && <span className="chip-btn selected">Custom</span>}
          </div>
          <div className="checks">
            <label className="inline muted">
              <input type="checkbox" checked disabled /> Busy blocks (always)
            </label>
            {VIS_FIELDS.map((f) => (
              <label className="inline" key={f.key} title={f.hint}>
                <input type="checkbox" checked={vis[f.key]} onChange={(e) => setVis({ ...vis, [f.key]: e.target.checked })} /> {f.label}
              </label>
            ))}
          </div>
        </fieldset>

        <fieldset>
          <legend>Which events</legend>
          <div className="checks">
            <div className="muted small">Calendars</div>
            {sources.map((s) => (
              <label className="inline" key={s.id}>
                <input type="checkbox" checked={filters.source_ids === null || filters.source_ids.includes(s.id)} onChange={() => toggleSource(s.id)} /> {s.name}{" "}
                <span className="muted small">({s.category})</span>
              </label>
            ))}
          </div>
          {categories.length > 1 && (
            <div className="checks">
              <div className="muted small">Categories</div>
              {categories.map((c) => (
                <label className="inline" key={c}>
                  <input type="checkbox" checked={filters.categories === null || filters.categories.includes(c)} onChange={() => toggleCategory(c)} /> {c}
                </label>
              ))}
            </div>
          )}
          <div className="checks">
            <div className="muted small">Include events where my RSVP is</div>
            {ALL_RSVP.map((r) => (
              <label className="inline" key={r}>
                <input type="checkbox" checked={filters.rsvp.includes(r)} onChange={() => toggleRsvp(r)} /> {r.replace("_", " ")}
              </label>
            ))}
          </div>
          <div className="checks">
            <label className="inline">
              <input type="checkbox" checked={filters.include_free} onChange={(e) => setFilters({ ...filters, include_free: e.target.checked })} /> Include events marked "free"
            </label>
            <label className="inline">
              <input type="checkbox" checked={filters.include_all_day} onChange={(e) => setFilters({ ...filters, include_all_day: e.target.checked })} /> Include all-day events
            </label>
            <label className="inline">
              <input type="checkbox" checked={filters.include_tentative} onChange={(e) => setFilters({ ...filters, include_tentative: e.target.checked })} /> Include tentative events
            </label>
          </div>
          <div className="row">
            <label>
              Days into the past
              <input type="number" min={0} max={3650} value={filters.horizon_past_days} onChange={(e) => setFilters({ ...filters, horizon_past_days: Number(e.target.value) })} />
            </label>
            <label>
              Days into the future
              <input type="number" min={1} max={3650} value={filters.horizon_future_days} onChange={(e) => setFilters({ ...filters, horizon_future_days: Number(e.target.value) })} />
            </label>
          </div>
        </fieldset>

        <div className="row end">
          <button type="button" className="secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" disabled={busy || (kind === "friend" && !friendId)}>
            {share ? "Save" : "Create share"}
          </button>
        </div>
      </form>
    </Modal>
  );
}

function PreviewModal({ share, onClose }: { share: Share; onClose: () => void }) {
  const from = new Date();
  const to = new Date(from.getTime() + 14 * 86400_000);
  const { data, error, loading } = useAsync(
    () => api.get<SharedEvent[]>(`/shares/${share.id}/preview?from=${from.toISOString()}&to=${to.toISOString()}`),
    [share.id],
  );
  return (
    <Modal title={`What "${share.name}" sees (next 14 days)`} onClose={onClose}>
      <ErrorBox error={error} />
      {loading && <div className="muted">Loading…</div>}
      {data && data.length === 0 && <Empty>Nothing would be shared in the next 14 days.</Empty>}
      <SharedEventTable events={data ?? []} />
    </Modal>
  );
}
