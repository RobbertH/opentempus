// A tiny SVG mock-UI kit used to illustrate the click path through other
// calendar providers.
//
// These are hand-drawn schematics, not captured screenshots: OpenTempus has no
// account on those services, real screenshots would age badly and carry other
// companies' trademarks. The labels and their position match the real UI, which
// is what you need to find the button.

export type Shape =
  | { t: "panel"; x: number; y: number; w: number; h: number; flat?: boolean }
  | { t: "text"; x: number; y: number; s: string; size?: number; bold?: boolean; muted?: boolean; mono?: boolean; anchor?: "start" | "middle" | "end" }
  | { t: "list"; x: number; y: number; w: number; items: string[]; active?: number; dots?: boolean }
  | { t: "menu"; x: number; y: number; w: number; items: string[]; hi?: number }
  | { t: "field"; x: number; y: number; w: number; s: string; label?: string; mono?: boolean; copy?: boolean }
  | { t: "select"; x: number; y: number; w: number; s: string; label?: string }
  | { t: "btn"; x: number; y: number; s: string; primary?: boolean; w?: number }
  | { t: "kebab"; x: number; y: number }
  | { t: "gear"; x: number; y: number }
  | { t: "ring"; x: number; y: number; w: number; h: number; n?: number; note?: string; below?: boolean }
  | { t: "cursor"; x: number; y: number };

export interface ShotSpec {
  /** Text shown in the fake address bar. Ignored when `bare` is set. */
  url: string;
  h: number;
  w?: number;
  /** Draw without browser chrome, for close-ups of a single control. */
  bare?: boolean;
  shapes: Shape[];
}

const DEFAULT_W = 560;
const ROW_H = 22;

/**
 * SVG has no text wrapping and no measurement before layout, so long values
 * would simply run past their box. Truncate in the middle with a conservative
 * per-character estimate instead.
 */
function fit(s: string, max: number, size: number, mono?: boolean): string {
  const per = size * (mono ? 0.82 : 0.62);
  const n = Math.floor(max / per);
  if (s.length <= n) return s;
  if (n <= 4) return s.slice(0, Math.max(1, n));
  const head = Math.ceil((n - 1) * 0.55);
  return `${s.slice(0, head)}…${s.slice(s.length - (n - 1 - head))}`;
}

export function Shot({ spec, caption }: { spec: ShotSpec; caption?: string }) {
  const w = spec.w ?? DEFAULT_W;
  return (
    <figure className="shot">
      <svg viewBox={`0 0 ${w} ${spec.h}`} role="img" aria-label={caption ?? spec.url}>
        <rect className="s-bg" x={0.5} y={0.5} width={w - 1} height={spec.h - 1} rx={8} />
        {!spec.bare && (
          <>
            <path className="s-chrome" d={`M 0.5 8.5 a 8 8 0 0 1 8 -8 h ${w - 17} a 8 8 0 0 1 8 8 v 20 h -${w - 1} z`} />
            {[13, 25, 37].map((cx) => (
              <circle key={cx} className="s-dot" cx={cx} cy={14.5} r={3.5} />
            ))}
            <rect className="s-url" x={54} y={7} width={w - 68} height={15} rx={7.5} />
            <text className="s-url-text" x={64} y={18}>
              {spec.url}
            </text>
          </>
        )}
        {spec.shapes.map((s, i) => (
          <Node key={i} s={s} />
        ))}
      </svg>
      {caption && <figcaption>{caption}</figcaption>}
    </figure>
  );
}

function Node({ s }: { s: Shape }) {
  switch (s.t) {
    case "panel":
      return <rect className={`s-panel${s.flat ? " flat" : ""}`} x={s.x} y={s.y} width={s.w} height={s.h} rx={6} />;

    case "text":
      return (
        <text
          className={`s-text${s.bold ? " bold" : ""}${s.muted ? " muted" : ""}${s.mono ? " mono" : ""}`}
          x={s.x}
          y={s.y}
          fontSize={s.size ?? 11}
          textAnchor={s.anchor ?? "start"}
        >
          {s.s}
        </text>
      );

    case "list":
      return (
        <g>
          {s.items.map((it, i) => (
            <g key={i}>
              {s.active === i && <rect className="s-active" x={s.x - 4} y={s.y + i * ROW_H - 13} width={s.w} height={ROW_H - 3} rx={5} />}
              {s.dots && <rect className="s-cal-dot" x={s.x} y={s.y + i * ROW_H - 9} width={8} height={8} rx={2} style={{ fill: CAL_COLORS[i % CAL_COLORS.length] }} />}
              <text className={`s-text${s.active === i ? " bold" : ""}`} x={s.x + (s.dots ? 16 : 0)} y={s.y + i * ROW_H} fontSize={11}>
                {it}
              </text>
            </g>
          ))}
        </g>
      );

    case "menu":
      return (
        <g>
          <rect className="s-menu" x={s.x} y={s.y} width={s.w} height={s.items.length * ROW_H + 8} rx={6} />
          {s.items.map((it, i) => (
            <g key={i}>
              {s.hi === i && <rect className="s-active" x={s.x + 3} y={s.y + 4 + i * ROW_H} width={s.w - 6} height={ROW_H} rx={4} />}
              <text className={`s-text${s.hi === i ? " bold" : ""}`} x={s.x + 10} y={s.y + 4 + i * ROW_H + 15} fontSize={11}>
                {it}
              </text>
            </g>
          ))}
        </g>
      );

    case "field":
      return (
        <g>
          {s.label && (
            <text className="s-text muted" x={s.x} y={s.y - 9} fontSize={10}>
              {s.label}
            </text>
          )}
          <rect className="s-input" x={s.x} y={s.y} width={s.w} height={24} rx={5} />
          <text className={`s-text${s.mono ? " mono" : ""}`} x={s.x + 8} y={s.y + 16} fontSize={s.mono ? 9.5 : 11}>
            {fit(s.s, s.w - 26 - (s.copy ? 26 : 0), s.mono ? 9.5 : 11, s.mono)}
          </text>
          {s.copy && (
            <g className="s-copy">
              <rect x={s.x + s.w - 26} y={s.y + 5} width={14} height={14} rx={3} />
              <rect x={s.x + s.w - 29} y={s.y + 8} width={14} height={14} rx={3} />
            </g>
          )}
        </g>
      );

    case "select":
      return (
        <g>
          {s.label && (
            <text className="s-text muted" x={s.x} y={s.y - 9} fontSize={10}>
              {s.label}
            </text>
          )}
          <rect className="s-input" x={s.x} y={s.y} width={s.w} height={24} rx={5} />
          <text className="s-text" x={s.x + 8} y={s.y + 16} fontSize={11}>
            {s.s}
          </text>
          <path className="s-chevron" d={`M ${s.x + s.w - 18} ${s.y + 10} l 5 5 l 5 -5`} />
        </g>
      );

    case "btn": {
      const w = s.w ?? Math.max(56, s.s.length * 6.6 + 20);
      return (
        <g>
          <rect className={`s-btn${s.primary ? " primary" : ""}`} x={s.x} y={s.y} width={w} height={24} rx={5} />
          <text className={`s-text${s.primary ? " on-accent" : ""}`} x={s.x + w / 2} y={s.y + 16} fontSize={11} textAnchor="middle">
            {s.s}
          </text>
        </g>
      );
    }

    case "kebab":
      return (
        <g className="s-glyph">
          {[0, 5, 10].map((dy) => (
            <circle key={dy} cx={s.x} cy={s.y + dy} r={1.6} />
          ))}
        </g>
      );

    case "gear":
      return (
        <g className="s-glyph">
          <circle cx={s.x} cy={s.y} r={5.5} fill="none" strokeWidth={1.6} />
          <circle cx={s.x} cy={s.y} r={1.8} />
          {[0, 45, 90, 135, 180, 225, 270, 315].map((a) => (
            <line key={a} x1={s.x + 6} y1={s.y} x2={s.x + 9} y2={s.y} strokeWidth={1.8} strokeLinecap="round" transform={`rotate(${a} ${s.x} ${s.y})`} />
          ))}
        </g>
      );

    case "ring":
      return (
        <g className="s-ring">
          <rect x={s.x} y={s.y} width={s.w} height={s.h} rx={6} />
          {s.n !== undefined && (
            <>
              <circle className="s-badge" cx={s.x - 10} cy={s.y + s.h / 2} r={9} />
              <text className="s-badge-text" x={s.x - 10} y={s.y + s.h / 2 + 4} textAnchor="middle" fontSize={11}>
                {s.n}
              </text>
            </>
          )}
          {s.note &&
            (s.below ? (
              <text className="s-note" x={s.x} y={s.y + s.h + 15} fontSize={10.5}>
                {s.note}
              </text>
            ) : (
              <text className="s-note" x={s.x + s.w + 10} y={s.y + s.h / 2 + 4} fontSize={10.5}>
                {s.note}
              </text>
            ))}
        </g>
      );

    case "cursor":
      return <path className="s-cursor" d={`M ${s.x} ${s.y} l 0 13 l 3.4 -3.4 l 2.3 5 l 2.6 -1.2 l -2.3 -4.9 l 4.7 -0.3 z`} />;
  }
}

const CAL_COLORS = ["#4f6df5", "#16a34a", "#db2777", "#ca8a04"];
