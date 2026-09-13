import { Navigate, NavLink, Route, Routes } from "react-router-dom";
import { AuthProvider, useAuth } from "./auth";
import CalendarPage from "./pages/CalendarPage";
import FlowsPage from "./pages/FlowsPage";
import TargetsPage from "./pages/TargetsPage";
import FriendsPage from "./pages/FriendsPage";
import LoginPage from "./pages/LoginPage";
import SharedWithMePage from "./pages/SharedWithMePage";
import SharesPage from "./pages/SharesPage";
import SourcesPage from "./pages/SourcesPage";
import SettingsPage from "./pages/SettingsPage";

function Shell() {
  const { user, loading, logout } = useAuth();
  if (loading) return <div className="center muted">Loading…</div>;
  if (!user) return <LoginPage />;
  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">◷</span> OpenTempus
        </div>
        <nav>
          <NavLink to="/" end>Calendar</NavLink>
          <NavLink to="/flows">Flows</NavLink>
          <div className="nav-group">In</div>
          <NavLink to="/sources">Calendars</NavLink>
          <div className="nav-group">Out</div>
          <NavLink to="/targets">Sync to calendars</NavLink>
          <NavLink to="/shares">Sharing</NavLink>
          <div className="nav-group">People</div>
          <NavLink to="/friends">Friends</NavLink>
          <NavLink to="/shared">Shared with me</NavLink>
          <NavLink to="/settings">Settings</NavLink>
        </nav>
        <div className="sidebar-footer">
          <div className="muted small">{user.display_name}</div>
          <button className="link" onClick={() => void logout()}>
            Sign out
          </button>
        </div>
      </aside>
      <main className="content">
        <Routes>
          <Route path="/" element={<CalendarPage />} />
          <Route path="/flows" element={<FlowsPage />} />
          <Route path="/sources" element={<SourcesPage />} />
          <Route path="/targets" element={<TargetsPage />} />
          <Route path="/shares" element={<SharesPage />} />
          <Route path="/friends" element={<FriendsPage />} />
          <Route path="/shared" element={<SharedWithMePage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </div>
  );
}

export default function App() {
  return (
    <AuthProvider>
      <Shell />
    </AuthProvider>
  );
}
