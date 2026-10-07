import { useEffect, useState } from "react";

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
  token,
  onConnectionSave,
}: {
  idleSecs: number;
  onIdleChange: (n: number) => void;
  connected: boolean;
  wsUrl: string;
  personaCount: number;
  token: string;
  onConnectionSave: (url: string, token: string) => void;
}) {
  const [endpoint, setEndpoint] = useState(wsUrl);
  const [accessToken, setAccessToken] = useState(token);
  const [provider, setProvider] = useState("ollama");
  const [model, setModel] = useState("");
  const [models, setModels] = useState<string[]>([]);
  const [apiKey, setApiKey] = useState("");
  const [notice, setNotice] = useState("");
  const [saving, setSaving] = useState(false);
  const base = wsUrl.replace(/^ws/, "http").replace(/\/api\/ws\/chat$/, "");
  const headers = token ? { Authorization: `Bearer ${token}` } : undefined;

  useEffect(() => {
    const controller = new AbortController();
    const read = async (key: string) => {
      const response = await fetch(`${base}/api/settings?key=${key}`, { headers, signal: controller.signal });
      if (!response.ok) throw new Error(`Engine settings: HTTP ${response.status}`);
      return await response.json() as string | null;
    };
    if (connected) {
      void (async () => {
        try {
          const selected = await read("llm_provider") || "ollama";
          const selectedModel = await read(selected === "ollama" ? "default_chat_model" : `${selected}_model`) || "";
          setProvider(selected);
          setModel(selectedModel);
          const response = await fetch(`${base}/api/models/available`, { headers, signal: controller.signal });
          if (response.ok) setModels(await response.json() as string[]);
        } catch (error) {
          if (!controller.signal.aborted) setNotice(String(error));
        }
      })();
    }
    return () => controller.abort();
  }, [connected, wsUrl, token]);

  async function saveModel() {
    if (!model.trim()) { setNotice("Choose or enter a model first."); return; }
    setSaving(true);
    setNotice("");
    try {
      const write = async (key: string, value: string) => {
        const response = await fetch(`${base}/api/settings`, {
          method: "POST", headers: { ...headers, "Content-Type": "application/json" },
          body: JSON.stringify({ key, value }), signal: AbortSignal.timeout(10000),
        });
        if (!response.ok) throw new Error(`Could not save settings: HTTP ${response.status}`);
      };
      await write(provider === "ollama" ? "default_chat_model" : `${provider}_model`, model.trim());
      if (provider === "ollama") await write("planner_model", model.trim());
      if (provider !== "ollama" && apiKey.trim()) await write(`${provider}_api_key`, apiKey.trim());
      await write("llm_provider", provider);
      setApiKey("");
      setNotice("Model settings saved. Your next message uses this provider.");
    } catch (error) { setNotice(String(error)); }
    finally { setSaving(false); }
  }

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
        <div className="setting-head"><h3>Connection</h3><p>Connect to your local engine. Enter its access token if enabled.</p></div>
        <label>Engine WebSocket<input className="form-input" value={endpoint} onChange={(event) => setEndpoint(event.target.value)} /></label>
        <label>Access token<input className="form-input" type="password" autoComplete="off" value={accessToken} onChange={(event) => setAccessToken(event.target.value)} /></label>
        <button className="seg-btn" onClick={() => {
          try {
            const parsed = new URL(endpoint);
            if (!["ws:", "wss:"].includes(parsed.protocol) || !["localhost", "127.0.0.1"].includes(parsed.hostname) || parsed.pathname !== "/api/ws/chat") throw new Error("Use a local WebSocket URL ending in /api/ws/chat.");
            onConnectionSave(endpoint, accessToken);
            setNotice("Connection saved.");
          } catch (error) { setNotice(String(error)); }
        }}>Save connection</button>
      </div>

      <div className="panel setting">
        <div className="setting-head"><h3>AI model</h3><p>Use an installed Ollama model or your cloud account.</p></div>
        <label>Provider<select className="form-input" value={provider} onChange={(event) => { setProvider(event.target.value); setModel(""); setApiKey(""); }}>
          <option value="ollama">Ollama · local</option><option value="openai">OpenAI · cloud</option><option value="anthropic">Anthropic · cloud</option>
        </select></label>
        {provider === "ollama" ? <label>Installed model<select className="form-input" value={model} onChange={(event) => setModel(event.target.value)}>
          <option value="">{models.length ? "Choose a model" : "No installed models found"}</option>
          {model && !models.includes(model) && <option value={model}>{model} (unavailable)</option>}
          {models.map((name) => <option key={name} value={name}>{name}</option>)}
        </select></label> : <>
          <label>Model ID<input className="form-input" value={model} onChange={(event) => setModel(event.target.value)} /></label>
          <label>API key<input className="form-input" type="password" autoComplete="off" placeholder="Leave blank to keep your saved key" value={apiKey} onChange={(event) => setApiKey(event.target.value)} /></label>
          <p className="setting-note">Cloud requests go to your selected provider. API keys are encrypted on this Mac.</p>
        </>}
        <button className="seg-btn" disabled={!connected || saving || (provider === "ollama" && !models.includes(model))} onClick={() => void saveModel()}>{saving ? "Saving…" : "Save model"}</button>
      </div>
      {notice && <p className="setting-note" role="status">{notice}</p>}

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
