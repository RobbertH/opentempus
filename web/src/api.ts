// Thin typed client for the OpenTempus API.

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`/api/v1${path}`, {
    method,
    headers: body !== undefined ? { "Content-Type": "application/json" } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
    credentials: "same-origin",
  });
  if (!res.ok) {
    let message = res.statusText;
    try {
      const data = await res.json();
      if (data && typeof data.error === "string") message = data.error;
    } catch {
      /* ignore */
    }
    throw new ApiError(res.status, message);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  get: <T>(path: string) => request<T>("GET", path),
  post: <T>(path: string, body?: unknown) => request<T>("POST", path, body ?? {}),
  patch: <T>(path: string, body: unknown) => request<T>("PATCH", path, body),
  delete: <T>(path: string) => request<T>("DELETE", path),
};

// ---- types -----------------------------------------------------------------

export interface User {
  id: string;
  email: string;
  display_name: string;
  timezone: string;
}

export type SourceKind = "ics_url" | "caldav" | "google" | "microsoft";
export type SyncStatus = "never" | "ok" | "error" | "running";

export interface CalDavConfig {
  url?: string;
  username?: string;
  password?: string;
  calendar_url?: string;
}

export interface Source {
  id: string;
  name: string;
  kind: SourceKind;
  config: CalDavConfig;
  category: string;
  color: string;
  horizon_past_days: number;
  horizon_future_days: number;
  sync_interval_secs: number;
  enabled: boolean;
  last_synced_at: string | null;
  last_sync_status: SyncStatus;
  last_sync_error: string | null;
  next_sync_at: string;
}

export type Rsvp = "organizer" | "accepted" | "tentative" | "declined" | "needs_action" | "unknown";

export interface MyEvent {
  id: string;
  event_id: string;
  source_id: string;
  source_name: string;
  category: string;
  color: string;
  start: string;
  end: string;
  all_day: boolean;
  title: string | null;
  description: string | null;
  location: string | null;
  status: "confirmed" | "tentative" | "cancelled";
  transparency: "opaque" | "transparent";
  rsvp: Rsvp;
}

export interface SharedEvent {
  id: string;
  start: string;
  end: string;
  all_day: boolean;
  busy: boolean;
  title?: string;
  description?: string;
  location?: string;
  category?: string;
  origin_calendar?: string;
  rsvp?: Rsvp;
}

export interface Visibility {
  title: boolean;
  description: boolean;
  location: boolean;
  category: boolean;
  origin_calendar: boolean;
  rsvp: boolean;
  free_busy: boolean;
}

export interface Filters {
  source_ids: string[] | null;
  categories: string[] | null;
  rsvp: Rsvp[];
  include_free: boolean;
  include_all_day: boolean;
  include_tentative: boolean;
  horizon_past_days: number;
  horizon_future_days: number;
}

export interface Share {
  id: string;
  name: string;
  audience_kind: "friend" | "link";
  audience_user_id: string | null;
  audience_display_name: string | null;
  audience_email: string | null;
  visibility: Visibility;
  filters: Filters;
  enabled: boolean;
  token: string;
  feed_url: string;
  api_url: string;
  created_at: string;
}

export interface Preset {
  id: string;
  label: string;
  description: string;
  visibility: Visibility;
}

export interface Friend {
  id: string;
  user_id: string;
  email: string;
  display_name: string;
  status: "pending" | "accepted" | "declined";
  incoming: boolean;
  created_at: string;
}

export interface SharedWithMe {
  id: string;
  name: string;
  owner_id: string;
  owner_display_name: string;
  owner_email: string;
  visibility: Visibility;
  feed_url: string;
  api_url: string;
  enabled: boolean;
}

export type TargetKind = "caldav" | "google" | "microsoft";

export interface Target {
  id: string;
  name: string;
  kind: TargetKind;
  config: CalDavConfig;
  visibility: Visibility;
  filters: Filters;
  placeholder_title: string;
  enabled: boolean;
  push_interval_secs: number;
  last_pushed_at: string | null;
  last_push_status: SyncStatus;
  last_push_error: string | null;
  mirrored_count: number;
  created_at: string;
}

export interface DiscoveredCalendar {
  url: string;
  name: string;
}

export const DEFAULT_FILTERS: Filters = {
  source_ids: null,
  categories: null,
  rsvp: ["organizer", "accepted", "tentative", "needs_action", "unknown"],
  include_free: false,
  include_all_day: true,
  include_tentative: true,
  horizon_past_days: 7,
  horizon_future_days: 90,
};

export const ALL_RSVP: Rsvp[] = ["organizer", "accepted", "tentative", "needs_action", "unknown", "declined"];
