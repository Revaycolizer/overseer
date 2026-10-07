import { useEffect, useState } from "react";

export default function Onboarding({ url, token, onComplete }: {url:string; token:string; onComplete:()=>void}) {
  const [loading,setLoading]=useState(true);
  const [provider,setProvider]=useState("ollama");
  const [model,setModel]=useState("");
  const [key,setKey]=useState("");
  const [models,setModels]=useState<string[]>([]);
  const [notice,setNotice]=useState("");
  const [busy,setBusy]=useState(false);
  const base=url.replace(/^ws/,"http").replace(/\/api\/ws\/chat$/,"");
  async function request(path:string, body?:unknown, signal?:AbortSignal) {
    const response=await fetch(base+path,{method:body?"POST":"GET",headers:{...(token?{Authorization:`Bearer ${token}`} : {}),...(body?{"Content-Type":"application/json"}:{})},body:body?JSON.stringify(body):undefined,signal:signal || AbortSignal.timeout(55000)});
    if(!response.ok)throw new Error((await response.text()) || `Request failed (${response.status})`);
    return response.status===204 ? null : response.text();
  }
  async function refreshModels() {
    try { const found=JSON.parse((await request("/api/models/available"))!);setModels(found);setNotice(found.length?"":"Ollama is running, but has no installed models. Install a model in Ollama, then refresh."); }
    catch {setModels([]);setNotice("Ollama is unavailable. Open Ollama on this computer, then refresh—or choose a cloud provider.");}
  }
  useEffect(()=>{
    const controller=new AbortController();
    void (async()=>{
      try {
        const complete=JSON.parse((await request("/api/settings?key=desktop_setup_complete",undefined,controller.signal))!);
        if(controller.signal.aborted)return;
        if(complete==="true"){onComplete();return;}
        const selected=JSON.parse((await request("/api/settings?key=llm_provider",undefined,controller.signal))!) || "ollama";
        const saved=JSON.parse((await request(`/api/settings?key=${selected==="ollama"?"default_chat_model":`${selected}_model`}`,undefined,controller.signal))!) || "";
        if(controller.signal.aborted)return;
        setProvider(["ollama","openai","anthropic"].includes(selected)?selected:"ollama");setModel(saved);
        if(selected==="ollama")await refreshModels();
      } catch(error){if(!controller.signal.aborted)setNotice(`Setup could not load: ${String(error)}`);}
      finally {if(!controller.signal.aborted)setLoading(false);}
    })();
    return ()=>controller.abort();
  },[url,token]);
  async function finish() {
    setBusy(true);setNotice("Checking your model…");
    try {
      await request("/api/setup/test",{provider,model:model.trim(),api_key:key});
      const write=(name:string,value:string)=>request("/api/settings",{key:name,value});
      await write(provider==="ollama"?"default_chat_model":`${provider}_model`,model.trim());
      if(provider==="ollama")await write("planner_model",model.trim());
      else if(key.trim())await write(`${provider}_api_key`,key.trim());
      await write("llm_provider",provider);
      await write("desktop_setup_complete","true");
      setKey("");onComplete();
    }catch(error){setNotice(String(error).replace(/^Error: /,""));}
    finally{setBusy(false);}
  }
  if(loading)return <div className="screen" role="status">Preparing your setup…</div>;
  return <div className="screen onboarding">
    <header className="screen-head"><div><h2 className="screen-title">Make Overseer yours</h2><p className="screen-sub">Choose where your AI runs. You can change this in Settings.</p></div></header>
    <form onSubmit={event=>{event.preventDefault();void finish();}}>
      <fieldset disabled={busy}>
        <legend className="setting-note">1. Choose a provider</legend>
        <label>AI provider<select className="form-input" value={provider} onChange={event=>{setProvider(event.target.value);setModel("");setKey("");setNotice("");if(event.target.value==="ollama")void refreshModels();}}>
          <option value="ollama">Ollama · on your computer</option><option value="openai">OpenAI · cloud</option><option value="anthropic">Anthropic · cloud</option>
        </select></label>
        <p className="setting-note">{provider==="ollama"?"Your prompts stay on this computer. Ollama and a downloaded model are required.":"Prompts go to your chosen provider. Your provider account is billed for usage, including this small connection test. API keys are encrypted by the local backend."}</p>
        <label>{provider==="ollama"?"2. Installed model":"2. Model ID"}{provider==="ollama"?<select className="form-input" value={model} onChange={event=>setModel(event.target.value)}><option value="">Choose an installed model</option>{model&&!models.includes(model)&&<option value={model}>{model} (unavailable)</option>}{models.map(name=><option key={name} value={name}>{name}</option>)}</select>:<input className="form-input" value={model} onChange={event=>setModel(event.target.value)} placeholder="Model ID from your provider account" required />}</label>
        {provider==="ollama"?<button type="button" className="seg-btn" onClick={()=>void refreshModels()}>Refresh installed models</button>:<label>API key<input className="form-input" type="password" autoComplete="off" value={key} onChange={event=>setKey(event.target.value)} placeholder="Enter a key, or leave blank to use your saved key" /></label>}
        <p className="setting-note">3. We’ll send a short, tool-free test message, then open your chat.</p>
        <button className="seg-btn on" type="submit" disabled={!model.trim()||(provider==="ollama"&&!models.includes(model))}>{busy?"Checking and saving…":"Check model and start chatting"}</button>
      </fieldset>
    </form>
    {notice&&<p className="setting-note" role="status" aria-live="polite">{notice}</p>}
    <button className="seg-btn" disabled={busy} onClick={onComplete}>Set up later</button>
  </div>;
}
