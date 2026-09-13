// The rule editor shared by shares and sync targets: what may be seen, and
// which events are included.

import { useMemo } from "react";
import { ALL_RSVP, type Filters, type Preset, type Rsvp, type Source, type Visibility } from "./api";

export const VIS_FIELDS: { key: keyof Visibility; label: string; hint: string }[] = [
  { key: "category", label: "Category", hint: "work / personal / … of the origin calendar" },
  { key: "origin_calendar", label: "Calendar name", hint: "which of your calendars the event is from" },
  { key: "title", label: "Title", hint: "the event's summary" },
  { key: "location", label: "Location", hint: "" },
  { key: "rsvp", label: "RSVP", hint: "whether you accepted, tentatively accepted, or have not replied" },
  { key: "free_busy", label: "Free/busy", hint: "distinguish events that do not actually block you" },
  { key: "description", label: "Description", hint: "full notes; often contains meeting links" },
];

export function presetFor(v: Visibility, presets: Preset[]): string {
  return presets.find((p) => JSON.stringify(p.visibility) === JSON.stringify(v))?.id ?? "custom";
}

export function VisibilityEditor({ vis, setVis, presets }: { vis: Visibility; setVis: (v: Visibility) => void; presets: Preset[] }) {
  const active = presetFor(vis, presets);
  return (
    <fieldset>
      <legend>What they may see</legend>
      <div className="row wrap">
        {presets.map((p) => (
          <button type="button" key={p.id} className={`chip-btn ${active === p.id ? "selected" : ""}`} onClick={() => setVis(p.visibility)} title={p.description}>
            {p.label}
          </button>
        ))}
        {active === "custom" && <span className="chip-btn selected">Custom</span>}
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
  );
}

export function FiltersEditor({ filters, setFilters, sources }: { filters: Filters; setFilters: (f: Filters) => void; sources: Source[] }) {
  const categories = useMemo(() => Array.from(new Set(sources.map((s) => s.category))).sort(), [sources]);

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

  return (
    <fieldset>
      <legend>Which events</legend>
      <div className="checks">
        <div className="muted small">Calendars</div>
        {sources.length === 0 && <span className="muted small">No calendars yet; all future calendars will be included.</span>}
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
  );
}

/** Human summary of a visibility mask. */
export function visibilityWords(v: Visibility): string[] {
  const parts: string[] = ["busy blocks"];
  if (v.category) parts.push("category");
  if (v.origin_calendar) parts.push("calendar name");
  if (v.title) parts.push("titles");
  if (v.location) parts.push("locations");
  if (v.rsvp) parts.push("RSVP");
  if (v.free_busy) parts.push("free/busy");
  if (v.description) parts.push("descriptions");
  return parts;
}

/** Human summary of filters. */
export function filterWords(f: Filters, sources: Source[]): string[] {
  const parts: string[] = [];
  if (f.source_ids) parts.push(`calendars: ${f.source_ids.map((id) => sources.find((s) => s.id === id)?.name ?? "?").join(", ")}`);
  else parts.push("all calendars");
  if (f.categories) parts.push(`categories: ${f.categories.join(", ")}`);
  if (f.rsvp.includes("declined")) parts.push("incl. declined");
  if (!f.rsvp.includes("needs_action")) parts.push("excl. unanswered");
  if (!f.rsvp.includes("tentative")) parts.push("excl. tentative RSVP");
  if (f.include_free) parts.push("incl. free time");
  if (!f.include_all_day) parts.push("no all-day");
  if (!f.include_tentative) parts.push("no tentative events");
  parts.push(`${f.horizon_past_days}d back / ${f.horizon_future_days}d ahead`);
  return parts;
}

/** Does this rule take events from this source? */
export function ruleIncludesSource(f: Filters, s: Source): boolean {
  if (f.source_ids && !f.source_ids.includes(s.id)) return false;
  if (f.categories && !f.categories.some((c) => c.toLowerCase() === s.category.toLowerCase())) return false;
  return true;
}
