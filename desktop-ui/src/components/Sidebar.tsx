export type View = "chat" | "studio" | "clock" | "settings";

const NAV: { id: View; label: string; path: string }[] = [
  { id: "chat", label: "Chat", path: "M4 5h16v10H7l-3 3z" },
  { id: "studio", label: "Studio", path: "M12 3l2.5 5.5L20 9l-4 4 1 6-5-3-5 3 1-6-4-4 5.5-.5z" },
  { id: "clock", label: "Clock", path: "M12 7v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z" },
  { id: "settings", label: "Settings", path: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 13a7.9 7.9 0 0 0 0-2l2-1.5-2-3.4-2.3 1a8 8 0 0 0-1.7-1L15 3H9l-.4 2.6a8 8 0 0 0-1.7 1l-2.3-1-2 3.4L2.6 11a7.9 7.9 0 0 0 0 2l-2 1.5 2 3.4 2.3-1a8 8 0 0 0 1.7 1L9 21h6l.4-2.6a8 8 0 0 0 1.7-1l2.3 1 2-3.4z" },
];

export default function Sidebar({
  view,
  onNavigate,
  collapsed,
  onToggle,
  connected,
}: {
  view: View;
  onNavigate: (v: View) => void;
  collapsed: boolean;
  onToggle: () => void;
  connected: boolean;
}) {
  return (
    <aside className={`sidebar ${collapsed ? "collapsed" : ""}`}>
      <div className="side-brand">
        <span className="mark" aria-hidden>
          <span className="mark-core" />
        </span>
        <span className="wordmark side-word">Overseer</span>
      </div>

      <div className="side-group">Workspace</div>
      <nav className="side-nav">
        {NAV.map((n) => (
          <button
            key={n.id}
            className={`side-item ${view === n.id ? "active" : ""}`}
            onClick={() => onNavigate(n.id)}
            title={n.label}
          >
            <span className="side-bar" aria-hidden />
            <svg viewBox="0 0 24 24" width="19" height="19" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
              <path d={n.path} />
            </svg>
            <span className="side-label">{n.label}</span>
          </button>
        ))}
      </nav>

      <div className="side-foot">
        <div className={`side-status ${connected ? "on" : "off"}`} title={connected ? "Engine online" : "Engine offline"}>
          <span className="pulse" />
          <span className="side-label">{connected ? "Engine online" : "Engine offline"}</span>
        </div>
        <button className="side-collapse" onClick={onToggle} title={collapsed ? "Expand" : "Collapse"}>
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
            <path d={collapsed ? "M9 6l6 6-6 6" : "M15 6l-6 6 6 6"} />
          </svg>
          <span className="side-label">Collapse</span>
        </button>
      </div>
    </aside>
  );
}
