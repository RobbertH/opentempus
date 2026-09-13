import { useState, type FormEvent } from "react";
import { api, type User } from "../api";
import { useAuth } from "../auth";
import { ErrorBox, Page } from "../components";

export default function SettingsPage() {
  const { user, setUser } = useAuth();
  const [name, setName] = useState(user?.display_name ?? "");
  const [tz, setTz] = useState(user?.timezone ?? "UTC");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setSaved(false);
    try {
      const u = await api.patch<User>("/auth/me", { display_name: name, timezone: tz });
      setUser(u);
      setSaved(true);
    } catch (err) {
      setError((err as Error).message);
    }
  }

  return (
    <Page title="Settings">
      <form className="card form narrow" onSubmit={submit}>
        <ErrorBox error={error} />
        <label>
          Email
          <input value={user?.email ?? ""} disabled />
        </label>
        <label>
          Display name
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <label>
          Time zone
          <input value={tz} onChange={(e) => setTz(e.target.value)} placeholder="Europe/Amsterdam" />
          <span className="hint">Used to interpret calendars that do not specify a zone. Browser: {Intl.DateTimeFormat().resolvedOptions().timeZone}</span>
        </label>
        <div className="row end">
          {saved && <span className="muted small">Saved</span>}
          <button type="submit">Save</button>
        </div>
      </form>
    </Page>
  );
}
