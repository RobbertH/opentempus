import { useState, type FormEvent } from "react";
import { api, type User } from "../api";
import { useAuth } from "../auth";
import { ErrorBox } from "../components";

export default function LoginPage() {
  const { setUser } = useAuth();
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      if (mode === "login") {
        const res = await api.post<{ user: User }>("/auth/login", { email, password });
        setUser(res.user);
      } else {
        const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
        const user = await api.post<User>("/auth/register", { email, password, display_name: name, timezone: tz });
        setUser(user);
      }
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="center">
      <form className="card auth-card" onSubmit={submit}>
        <div className="brand large">
          <span className="brand-mark">◷</span> OpenTempus
        </div>
        <p className="muted">Sync your calendars. Share exactly as much as you want.</p>
        <ErrorBox error={error} />
        {mode === "register" && (
          <label>
            Name
            <input value={name} onChange={(e) => setName(e.target.value)} required autoComplete="name" />
          </label>
        )}
        <label>
          Email
          <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required autoComplete="email" />
        </label>
        <label>
          Password
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={8}
            autoComplete={mode === "login" ? "current-password" : "new-password"}
          />
        </label>
        <button type="submit" disabled={busy}>
          {mode === "login" ? "Sign in" : "Create account"}
        </button>
        <button type="button" className="link" onClick={() => setMode(mode === "login" ? "register" : "login")}>
          {mode === "login" ? "No account yet? Create one" : "Already have an account? Sign in"}
        </button>
      </form>
    </div>
  );
}
