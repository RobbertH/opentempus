// The flow map: every inbound calendar on the left, OpenTempus in the middle,
// every outbound rule (sync target, friend share, secret link) on the right.
// Click a node to see what feeds it, or what it feeds.

import { useMemo, useState } from "react";
import { api, type Filters, type Preset, type Share, type Source, type Target, type Visibility } from "../api";
import { Badge, ErrorBox, Page, useAsync } from "../components";
import { filterWords, presetFor, ruleIncludesSource, visibilityWords } from "../RuleEditor";

type Mode = "flows" | "access";

interface Rule {
  id: string;
  kind: "target" | "friend" | "link";
  name: string;
  visibility: Visibility;
  filters: Filters;
  enabled: boolean;
  /** Where it ends up: calendar URL, friend name, or "secret link". */
  destination: string;
  audienceId: string; // for grouping in access mode
  audienceName: string;
  detail: string[];
}

interface OutNode {
  id: string;
  kind: "target" | "friend" | "link" | "person";
  label: string;
  sub: string;
  rules: Rule[];
  disabled: boolean;
}

const W = 1000;
const NODE_W = 280;
const NODE_H = 52;
const GAP = 12;
const PAD = 24;
const HUB_W = 150;
const HUB_H = 64;

export default function FlowsPage() {
  const sources = useAsync(() => api.get<Source[]>("/sources"), []);
  const targets = useAsync(() => api.get<Target[]>("/targets"), []);
  const shares = useAsync(() => api.get<Share[]>("/shares"), []);
  const presets = useAsync(() => api.get<Preset[]>("/shares/presets"), []);
  const [mode, setMode] = useState<Mode>("flows");
  const [selected, setSelected] = useState<{ side: "in" | "out" | "hub"; id: string } | null>(null);

  const error = sources.error ?? targets.error ?? shares.error ?? presets.error;
  const src = sources.data ?? [];
  const pre = presets.data ?? [];

  const rules: Rule[] = useMemo(() => {
    const out: Rule[] = [];
    for (const t of targets.data ?? []) {
      out.push({
        id: `t:${t.id}`,
        kind: "target",
        name: t.name,
        visibility: t.visibility,
        filters: t.filters,
        enabled: t.enabled,
        destination: t.config.calendar_url ?? t.kind,
        audienceId: `cal:${t.config.calendar_url ?? t.id}`,
        audienceName: `Calendar ${calendarPath(t.config.calendar_url ?? "")}`,
        detail: [`writes into ${t.config.calendar_url ?? t.kind}`, `${t.mirrored_count} events mirrored`],
      });
    }
    for (const s of shares.data ?? []) {
      const friend = s.audience_kind === "friend";
      out.push({
        id: `s:${s.id}`,
        kind: friend ? "friend" : "link",
        name: s.name,
        visibility: s.visibility,
        filters: s.filters,
        enabled: s.enabled,
        destination: friend ? (s.audience_display_name ?? "friend") : "secret link",
        audienceId: friend ? `u:${s.audience_user_id}` : `l:${s.id}`,
        audienceName: friend ? (s.audience_display_name ?? "friend") : `${s.name} (secret link)`,
        detail: [friend ? `shared with ${s.audience_display_name} (${s.audience_email})` : "anyone holding the secret link, e.g. an AI agent", `feed: ${s.feed_url}`],
      });
    }
    return out;
  }, [targets.data, shares.data]);

  const outNodes: OutNode[] = useMemo(() => {
    if (mode === "flows") {
      return rules.map((r) => ({
        id: r.id,
        kind: r.kind,
        label: r.name,
        sub: r.kind === "target" ? `→ ${shortUrl(r.destination)}` : r.kind === "friend" ? `→ ${r.destination}` : "→ secret link / agent",
        rules: [r],
        disabled: !r.enabled,
      }));
    }
    const groups = new Map<string, OutNode>();
    for (const r of rules) {
      const g = groups.get(r.audienceId);
      if (g) {
        g.rules.push(r);
        g.disabled = g.disabled && !r.enabled;
      } else {
        groups.set(r.audienceId, {
          id: r.audienceId,
          kind: r.kind === "target" ? "target" : r.kind === "friend" ? "person" : "link",
          label: r.audienceName,
          sub: "",
          rules: [r],
          disabled: !r.enabled,
        });
      }
    }
    for (const g of groups.values()) {
      const fields = new Set<string>();
      for (const r of g.rules) for (const w of visibilityWords(r.visibility)) fields.add(w);
      g.sub = `sees: ${Array.from(fields).join(", ")}`;
    }
    return Array.from(groups.values());
  }, [rules, mode]);

  // Which sources feed which out node.
  const feeds = useMemo(() => {
    const m = new Map<string, Set<string>>();
    for (const o of outNodes) {
      const set = new Set<string>();
      for (const r of o.rules) for (const s of src) if (ruleIncludesSource(r.filters, s)) set.add(s.id);
      m.set(o.id, set);
    }
    return m;
  }, [outNodes, src]);

  // Two-way: a CalDAV source that is also a target destination.
  const twoWay = useMemo(() => {
    const m = new Map<string, string>(); // source id -> target rule id, and target rule id -> source id
    for (const s of src) {
      if (s.kind !== "caldav" || !s.config.calendar_url) continue;
      for (const r of rules) if (r.kind === "target" && r.destination === s.config.calendar_url) {
        m.set(s.id, r.id);
        m.set(r.id, s.id);
      }
    }
    return m;
  }, [src, rules]);

  const activeIn = new Set<string>();
  const activeOut = new Set<string>();
  if (!selected || selected.side === "hub") {
    src.forEach((s) => activeIn.add(s.id));
    outNodes.forEach((o) => activeOut.add(o.id));
  } else if (selected.side === "in") {
    activeIn.add(selected.id);
    for (const o of outNodes) if (feeds.get(o.id)?.has(selected.id)) activeOut.add(o.id);
  } else {
    activeOut.add(selected.id);
    for (const id of feeds.get(selected.id) ?? []) activeIn.add(id);
  }

  const rows = Math.max(src.length, outNodes.length, 1);
  const H = Math.max(PAD * 2 + rows * (NODE_H + GAP), 260);
  const leftX = PAD;
  const rightX = W - PAD - NODE_W;
  const hubX = W / 2 - HUB_W / 2;
  const hubY = H / 2 - HUB_H / 2;
  const yAt = (i: number, n: number) => PAD + (H - PAD * 2 - n * (NODE_H + GAP) + GAP) / 2 + i * (NODE_H + GAP);

  const selectedSource = selected?.side === "in" ? src.find((s) => s.id === selected.id) : undefined;
  const selectedOut = selected?.side === "out" ? outNodes.find((o) => o.id === selected.id) : undefined;

  return (
    <Page
      title="Flows"
      actions={
        <div className="row">
          <button className={`chip-btn ${mode === "flows" ? "selected" : ""}`} onClick={() => { setMode("flows"); setSelected(null); }}>
            By rule
          </button>
          <button className={`chip-btn ${mode === "access" ? "selected" : ""}`} onClick={() => { setMode("access"); setSelected(null); }}>
            Who has access
          </button>
        </div>
      }
    >
      <p className="muted">
        {mode === "flows"
          ? "Calendars flow in on the left, rules send filtered views out on the right. Click a node to highlight what it feeds or what feeds it."
          : "Everyone and everything that can see your time, and what exactly they see. Click a person or calendar to see which of your calendars they get."}
      </p>
      <ErrorBox error={error} />
      <div className="flow-wrap">
        <svg viewBox={`0 0 ${W} ${H}`} className="flow" role="img" aria-label="Calendar flow map">
          <defs>
            <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
              <path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor" />
            </marker>
          </defs>
          {/* edges in */}
          {src.map((s, i) => {
            const y = yAt(i, src.length) + NODE_H / 2;
            const on = activeIn.has(s.id) && (activeOut.size > 0 || !selected);
            return (
              <g key={`e-in-${s.id}`} className={`edge ${on ? "on" : "off"}`}>
                <path d={curve(leftX + NODE_W, y, hubX, hubY + HUB_H / 2)} markerEnd="url(#arrow)" />
                <EdgeLabel {...bezierAt(leftX + NODE_W, y, hubX, hubY + HUB_H / 2, 0.3)} text={s.category} />
              </g>
            );
          })}
          {/* edges out */}
          {outNodes.map((o, i) => {
            const y = yAt(i, outNodes.length) + NODE_H / 2;
            const on = activeOut.has(o.id) && (activeIn.size > 0 || !selected);
            const label = o.rules.length === 1 ? presetLabel(o.rules[0].visibility, pre) : `${o.rules.length} rules`;
            return (
              <g key={`e-out-${o.id}`} className={`edge ${on ? "on" : "off"} ${o.disabled ? "disabled" : ""}`}>
                <path d={curve(hubX + HUB_W, hubY + HUB_H / 2, rightX, y)} markerEnd="url(#arrow)" />
                <EdgeLabel {...bezierAt(hubX + HUB_W, hubY + HUB_H / 2, rightX, y, 0.7)} text={label} />
              </g>
            );
          })}
          {/* hub */}
          <g className={`node hub ${selected?.side === "hub" ? "selected" : ""}`} onClick={() => setSelected(selected?.side === "hub" ? null : { side: "hub", id: "hub" })}>
            <rect x={hubX} y={hubY} width={HUB_W} height={HUB_H} rx={12} />
            <text x={hubX + HUB_W / 2} y={hubY + 28} textAnchor="middle" className="title">
              ◷ OpenTempus
            </text>
            <text x={hubX + HUB_W / 2} y={hubY + 46} textAnchor="middle" className="sub">
              {src.length} in · {rules.length} out
            </text>
          </g>
          {/* sources */}
          {src.map((s, i) => {
            const y = yAt(i, src.length);
            const on = activeIn.has(s.id);
            return (
              <g
                key={s.id}
                className={`node ${on ? "on" : "off"} ${selected?.side === "in" && selected.id === s.id ? "selected" : ""}`}
                onClick={() => setSelected(selected?.side === "in" && selected.id === s.id ? null : { side: "in", id: s.id })}
              >
                <rect x={leftX} y={y} width={NODE_W} height={NODE_H} rx={10} />
                <rect x={leftX} y={y} width={6} height={NODE_H} rx={3} fill={s.color} />
                <text x={leftX + 16} y={y + 22} className="title">
                  {trunc(s.name, 30)}
                </text>
                <text x={leftX + 16} y={y + 40} className="sub">
                  {s.category} · {s.kind === "ics_url" ? "ICS feed" : s.kind}
                  {twoWay.has(s.id) ? " · ⇄ two-way" : ""}
                  {!s.enabled ? " · paused" : ""}
                </text>
              </g>
            );
          })}
          {/* outbound */}
          {outNodes.map((o, i) => {
            const y = yAt(i, outNodes.length);
            const on = activeOut.has(o.id);
            return (
              <g
                key={o.id}
                className={`node ${o.kind} ${on ? "on" : "off"} ${selected?.side === "out" && selected.id === o.id ? "selected" : ""}`}
                onClick={() => setSelected(selected?.side === "out" && selected.id === o.id ? null : { side: "out", id: o.id })}
              >
                <rect x={rightX} y={y} width={NODE_W} height={NODE_H} rx={10} />
                <text x={rightX + 14} y={y + 22} className="title">
                  {icon(o.kind)} {trunc(o.label, 30)}
                </text>
                <text x={rightX + 14} y={y + 40} className="sub">
                  {trunc(o.sub, 44)}
                  {o.rules.some((r) => twoWay.has(r.id)) ? " · ⇄" : ""}
                  {o.disabled ? " · disabled" : ""}
                </text>
              </g>
            );
          })}
          {sources.data && src.length === 0 && (
            <text x={leftX + NODE_W / 2} y={H / 2} textAnchor="middle" className="sub">
              no calendars yet
            </text>
          )}
          {targets.data && shares.data && outNodes.length === 0 && (
            <text x={rightX + NODE_W / 2} y={H / 2} textAnchor="middle" className="sub">
              nothing goes out yet
            </text>
          )}
        </svg>
      </div>

      {selectedSource && (
        <div className="card detail">
          <h3>
            <span className="swatch-inline" style={{ background: selectedSource.color }} /> {selectedSource.name}
          </h3>
          <div className="muted small">
            Inbound · {selectedSource.kind === "ics_url" ? "ICS feed" : selectedSource.kind} · category <Badge>{selectedSource.category}</Badge>
            {twoWay.has(selectedSource.id) && <> · two-way: also receives mirrored events from rule "{rules.find((r) => r.id === twoWay.get(selectedSource.id))?.name}"</>}
          </div>
          <h4>Goes to</h4>
          {outNodes.filter((o) => feeds.get(o.id)?.has(selectedSource.id)).length === 0 && <div className="muted small">Nowhere. No rule includes this calendar.</div>}
          <ul className="plain">
            {outNodes
              .filter((o) => feeds.get(o.id)?.has(selectedSource.id))
              .map((o) => (
                <li key={o.id}>
                  <b>{icon(o.kind)} {o.label}</b>
                  {o.rules.map((r) => (
                    <div key={r.id} className="muted small">
                      {o.rules.length > 1 && <>{r.name}: </>}
                      sees {visibilityWords(r.visibility).join(", ")} · {filterWords(r.filters, src).join(" · ")}
                    </div>
                  ))}
                </li>
              ))}
          </ul>
        </div>
      )}
      {selectedOut && (
        <div className="card detail">
          <h3>
            {icon(selectedOut.kind)} {selectedOut.label}
          </h3>
          {selectedOut.rules.map((r) => (
            <div key={r.id} className="rule-block">
              {selectedOut.rules.length > 1 && <div><b>{r.name}</b>{!r.enabled && <Badge tone="warn">disabled</Badge>}</div>}
              <div className="row wrap">
                <span className="muted small">Sees:</span>
                {visibilityWords(r.visibility).map((w) => (
                  <Badge key={w} tone="ok">
                    {w}
                  </Badge>
                ))}
                <span className="muted small">Hidden:</span>
                {hiddenWords(r.visibility).map((w) => (
                  <Badge key={w}>{w}</Badge>
                ))}
              </div>
              <div className="muted small">Filters: {filterWords(r.filters, src).join(" · ")}</div>
              {r.detail.map((d) => (
                <div key={d} className="muted small ellipsis">
                  {d}
                </div>
              ))}
            </div>
          ))}
          <h4>Fed by</h4>
          {(feeds.get(selectedOut.id)?.size ?? 0) === 0 && <div className="muted small">No calendar matches these filters.</div>}
          <ul className="plain">
            {src
              .filter((s) => feeds.get(selectedOut.id)?.has(s.id))
              .map((s) => (
                <li key={s.id}>
                  <span className="swatch-inline" style={{ background: s.color }} /> <b>{s.name}</b> <span className="muted small">({s.category})</span>
                </li>
              ))}
          </ul>
        </div>
      )}
    </Page>
  );
}

function EdgeLabel({ x, y, text }: { x: number; y: number; text: string }) {
  const w = Math.max(28, text.length * 6.4 + 12);
  return (
    <g className="edge-label">
      <rect x={x - w / 2} y={y - 9} width={w} height={18} rx={9} />
      <text x={x} y={y + 4} textAnchor="middle">
        {text}
      </text>
    </g>
  );
}

function curve(x1: number, y1: number, x2: number, y2: number): string {
  const dx = (x2 - x1) / 2;
  return `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
}

/** Point at parameter t on the same cubic curve() draws. */
function bezierAt(x1: number, y1: number, x2: number, y2: number, t: number): { x: number; y: number } {
  const dx = (x2 - x1) / 2;
  const [cx1, cy1, cx2, cy2] = [x1 + dx, y1, x2 - dx, y2];
  const u = 1 - t;
  return {
    x: u * u * u * x1 + 3 * u * u * t * cx1 + 3 * u * t * t * cx2 + t * t * t * x2,
    y: u * u * u * y1 + 3 * u * u * t * cy1 + 3 * u * t * t * cy2 + t * t * t * y2,
  };
}

function trunc(s: string, n: number): string {
  return s.length > n ? s.slice(0, n - 1) + "…" : s;
}

function shortUrl(u: string): string {
  try {
    const p = new URL(u);
    return `${p.host}${p.pathname}`;
  } catch {
    return u;
  }
}

function calendarPath(u: string): string {
  try {
    const p = new URL(u).pathname.replace(/\/$/, "");
    const parts = p.split("/").filter(Boolean);
    return parts.slice(-2).join("/") || u;
  } catch {
    return u;
  }
}

function icon(kind: OutNode["kind"]): string {
  switch (kind) {
    case "target":
      return "📅";
    case "friend":
    case "person":
      return "👤";
    case "link":
      return "🔗";
  }
}

function presetLabel(v: Visibility, presets: Preset[]): string {
  const id = presetFor(v, presets);
  return presets.find((p) => p.id === id)?.label.toLowerCase() ?? "custom";
}

function hiddenWords(v: Visibility): string[] {
  const all: [keyof Visibility, string][] = [
    ["category", "category"],
    ["origin_calendar", "calendar name"],
    ["title", "titles"],
    ["location", "locations"],
    ["rsvp", "RSVP"],
    ["free_busy", "free/busy"],
    ["description", "descriptions"],
  ];
  return all.filter(([k]) => !v[k]).map(([, w]) => w);
}
