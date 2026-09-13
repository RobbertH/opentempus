import { useEffect, useState, type FormEvent } from "react";
import { DEFAULT_FILTERS, api, type Filters, type Friend, type Preset, type Share, type SharedEvent, type Source, type Visibility } from "../api";
import { Badge, CopyButton, Empty, ErrorBox, Modal, Page, useAsync } from "../components";
import { FiltersEditor, VisibilityEditor, filterWords, visibilityWords } from "../RuleEditor";
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
  return <>Shows: {visibilityWords(v).join(", ")}</>;
}

function FilterSummary({ f, sources }: { f: Filters; sources: Source[] }) {
  return <>{filterWords(f, sources).join(" · ")}</>;
}

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

  useEffect(() => {
    if (!name && kind === "friend") {
      const f = friends.find((x) => x.user_id === friendId);
      if (f) setName(`For ${f.display_name}`);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [friendId, kind]);

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

        <VisibilityEditor vis={vis} setVis={setVis} presets={presets} />
        <FiltersEditor filters={filters} setFilters={setFilters} sources={sources} />

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
