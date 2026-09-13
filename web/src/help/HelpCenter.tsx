// The help overlay behind the "?" icon: how to get a calendar address or an
// app password out of the services people actually use.

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

import { Badge, CopyButton, Modal } from "../components";
import { GUIDES, guideById, type Guide } from "./guides";
import { Shot } from "./Shot";

interface HelpApi {
  /** Open the help overlay, optionally straight to one provider's guide. */
  openHelp: (id?: string) => void;
}

const HelpContext = createContext<HelpApi>({ openHelp: () => {} });

export function useHelp(): HelpApi {
  return useContext(HelpContext);
}

export function HelpProvider({ children }: { children: ReactNode }) {
  const [shown, setShown] = useState(false);
  const [id, setId] = useState<string | null>(null);
  const openHelp = useCallback((next?: string) => {
    setId(next ?? null);
    setShown(true);
  }, []);
  const api = useMemo(() => ({ openHelp }), [openHelp]);
  const guide = id ? guideById(id) : undefined;

  return (
    <HelpContext.Provider value={api}>
      {children}
      {shown && (
        <Modal title={guide ? `Connecting ${guide.name}` : "Connecting a calendar"} wide onClose={() => setShown(false)}>
          {guide ? <GuideView guide={guide} onBack={() => setId(null)} /> : <GuideIndex onPick={setId} />}
        </Modal>
      )}
    </HelpContext.Provider>
  );
}

/** Inline "how do I find this?" link that jumps into a guide. */
export function HelpLink({ id, children }: { id?: string; children: ReactNode }) {
  const { openHelp } = useHelp();
  return (
    <button type="button" className="link" onClick={() => openHelp(id)}>
      {children}
    </button>
  );
}

/** The "?" button in the sidebar. */
export function HelpButton() {
  const { openHelp } = useHelp();
  return (
    <button type="button" className="help-btn" title="Help: connecting calendars" aria-label="Help" onClick={() => openHelp()}>
      ?
    </button>
  );
}

function GuideIndex({ onPick }: { onPick: (id: string) => void }) {
  return (
    <div className="help-index">
      <p className="muted">
        Pick your calendar service. Every guide ends with the exact values to type into OpenTempus.
      </p>
      <div className="help-grid">
        {GUIDES.map((g) => (
          <button key={g.id} type="button" className="help-card" onClick={() => onPick(g.id)}>
            <span className="tile" style={{ background: g.tone }}>
              {g.tile}
            </span>
            <span className="grow">
              <span className="row">
                <b>{g.name}</b>
                {g.twoWay ? <Badge tone="ok">two-way</Badge> : <Badge>read-only</Badge>}
              </span>
              <span className="muted small">{g.summary}</span>
            </span>
          </button>
        ))}
      </div>
      <div className="help-legend">
        <p>
          <b>Read-only</b> means OpenTempus can pull events in but cannot write anything back, because the service offers
          no writable protocol. You can still share those events with friends and agents, and mirror them into any
          calendar that <b>is</b> writable.
        </p>
        <p>
          <b>Two-way</b> means the service speaks CalDAV, so the same account can be both a source and a sync target.
        </p>
        <p className="muted small">
          The pictures below are drawings of each service's screens, not screenshots. Labels and their positions follow
          the real interface; if a provider moves something, the linked official page is the authority.
        </p>
      </div>
    </div>
  );
}

function GuideView({ guide, onBack }: { guide: Guide; onBack: () => void }) {
  return (
    <div className="help-guide">
      <div className="row">
        <button type="button" className="secondary small" onClick={onBack}>
          ‹ All services
        </button>
        {guide.twoWay ? <Badge tone="ok">two-way sync</Badge> : <Badge>read-only feed</Badge>}
        <Badge>{guide.kind === "ics" ? "iCal / ICS address" : "CalDAV account"}</Badge>
      </div>

      <ol className="help-steps">
        {guide.steps.map((s, i) => (
          <li key={i}>
            <div className="help-step-body">
              <div className="help-step-text">{s.text}</div>
              {s.shot && <Shot spec={s.shot} caption={s.caption} />}
              {!s.shot && s.caption && <div className="muted small">{s.caption}</div>}
            </div>
          </li>
        ))}
      </ol>

      <h4>What to enter in OpenTempus</h4>
      <table className="table">
        <tbody>
          {guide.fill.map((f) => (
            <tr key={f.label}>
              <td className="nowrap muted">{f.label}</td>
              <td>
                <div className="row">
                  <span className={f.copy ? "mono" : ""}>{f.value}</span>
                  {f.copy && <CopyButton text={f.value} />}
                </div>
                {f.note && <div className="muted small">{f.note}</div>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h4>Worth knowing</h4>
      <ul className="help-caveats">
        {guide.caveats.map((c) => (
          <li key={c}>{c}</li>
        ))}
      </ul>

      <h4>Official documentation</h4>
      <ul className="plain">
        {guide.docs.map((d) => (
          <li key={d.url}>
            <a href={d.url} target="_blank" rel="noreferrer noopener">
              {d.label}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
