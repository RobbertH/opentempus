import { useState, type FormEvent } from "react";
import { api, type Friend } from "../api";
import { Badge, Empty, ErrorBox, Page, useAsync } from "../components";

export default function FriendsPage() {
  const { data, error, reload } = useAsync(() => api.get<Friend[]>("/friends"), []);
  const [email, setEmail] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  async function send(e: FormEvent) {
    e.preventDefault();
    setErr(null);
    setMsg(null);
    try {
      const res = await api.post<{ status: string }>("/friends", { email });
      setMsg(res.status === "accepted" ? "You are now friends." : "Request sent (if that address has an account).");
      setEmail("");
      reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }

  async function act(path: string) {
    setErr(null);
    try {
      await api.post(path);
      reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }

  async function remove(f: Friend) {
    if (!window.confirm(`Remove ${f.display_name}? Shares between you will be deleted.`)) return;
    try {
      await api.delete(`/friends/${f.id}`);
      reload();
    } catch (e) {
      setErr((e as Error).message);
    }
  }

  const incoming = data?.filter((f) => f.status === "pending" && f.incoming) ?? [];
  const outgoing = data?.filter((f) => f.status === "pending" && !f.incoming) ?? [];
  const accepted = data?.filter((f) => f.status === "accepted") ?? [];

  return (
    <Page title="Friends">
      <p className="muted">Friends are people you can share calendars with. Each friend gets their own sharing rule, so you decide per person what they see.</p>
      <form className="card row" onSubmit={send}>
        <input type="email" placeholder="friend@example.com" value={email} onChange={(e) => setEmail(e.target.value)} required className="grow" />
        <button type="submit">Send friend request</button>
      </form>
      {msg && <div className="ok-box">{msg}</div>}
      <ErrorBox error={error ?? err} />

      {incoming.length > 0 && (
        <>
          <h2>Requests for you</h2>
          <div className="list">
            {incoming.map((f) => (
              <div className="card row-card" key={f.id}>
                <div className="grow">
                  <b>{f.display_name}</b> <span className="muted small">{f.email}</span>
                </div>
                <button className="small" onClick={() => void act(`/friends/${f.id}/accept`)}>
                  Accept
                </button>
                <button className="secondary small" onClick={() => void act(`/friends/${f.id}/decline`)}>
                  Decline
                </button>
              </div>
            ))}
          </div>
        </>
      )}

      <h2>Friends</h2>
      {accepted.length === 0 && <Empty>No friends yet.</Empty>}
      <div className="list">
        {accepted.map((f) => (
          <div className="card row-card" key={f.id}>
            <div className="grow">
              <b>{f.display_name}</b> <span className="muted small">{f.email}</span>
            </div>
            <button className="danger small" onClick={() => void remove(f)}>
              Remove
            </button>
          </div>
        ))}
      </div>

      {outgoing.length > 0 && (
        <>
          <h2>Sent requests</h2>
          <div className="list">
            {outgoing.map((f) => (
              <div className="card row-card" key={f.id}>
                <div className="grow">
                  <b>{f.display_name}</b> <span className="muted small">{f.email}</span> <Badge>pending</Badge>
                </div>
                <button className="secondary small" onClick={() => void remove(f)}>
                  Withdraw
                </button>
              </div>
            ))}
          </div>
        </>
      )}
    </Page>
  );
}
