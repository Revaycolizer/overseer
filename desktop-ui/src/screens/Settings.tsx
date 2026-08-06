/**
 * Settings — currently local/client preferences. Engine-side settings (provider
 * keys, supervision mode) live in the daemon; surfaced here read-only for now.
 */
export default function Settings({
  idleSecs,
  onIdleChange,
  connected,
  wsUrl,
  personaCount,
}: {
  idleSecs: number;
  onIdleChange: (n: number) => void;
  connected: boolean;
  wsUrl: string;
  personaCount: number;
}) {
  const options = [
    { v: 0, label: "Off" },
    { v: 60, label: "1 min" },
    { v: 300, label: "5 min" },
    { v: 600, label: "10 min" },
    { v: 1800, label: "30 min" },
  ];

  return (
    <div className="screen">
      <header className="screen-head">
        <div>
          <h2 className="screen-title">Settings</h2>
          <p className="screen-sub">Preferences for this device.</p>
        </div>
      </header>

      <div className="panel setting">
        <div className="setting-head">
          <h3>Inactivity clock</h3>
          <p>After this long with no mouse or keyboard, the screen becomes an ambient clock.</p>
        </div>
        <div className="seg">
          {options.map((o) => (
            <button
              key={o.v}
              className={`seg-btn ${idleSecs === o.v ? "on" : ""}`}
              onClick={() => onIdleChange(o.v)}
            >
              {o.label}
            </button>
          ))}
        </div>
      </div>

      <div className="panel setting">
        <div className="setting-head">
          <h3>Engine</h3>
          <p>The desktop app talks to the local Overseer daemon.</p>
        </div>
        <div className="kv">
          <div>
            <span className="k">Status</span>
            <span className={`v ${connected ? "ok" : "bad"}`}>{connected ? "connected" : "offline"}</span>
          </div>
          <div>
            <span className="k">WebSocket</span>
            <span className="v mono">{wsUrl}</span>
          </div>
          <div>
            <span className="k">Mini-GPTs</span>
            <span className="v">{personaCount}</span>
          </div>
        </div>
        {!connected && (
          <p className="setting-note">
            Not connected? Start the engine in a terminal: <code>overseer --server</code>
          </p>
        )}
      </div>

      <div className="panel setting">
        <div className="setting-head">
          <h3>Autonomy</h3>
          <p>
            Actions are governed by the Kernel. Set the supervision mode on the engine:
            <code>approval_required</code> (default), <code>autonomous</code> (auto-approve safe
            reads), or <code>full_auto</code>.
          </p>
        </div>
      </div>
    </div>
  );
}
