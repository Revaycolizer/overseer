import {fileURLToPath} from "node:url";
import {spawn} from 'node:child_process';
import {mkdtemp,rm} from 'node:fs/promises';
const root=fileURLToPath(new URL('..', import.meta.url)).replace(/\/$/, '');
const dir=await mkdtemp('/private/tmp/overseer-desktop-history-');
const token='isolated-desktop-history-verification';
const daemon=spawn(root+'/target/release/overseer',['--server','--port','18467'],{cwd:dir,env:{...process.env,OVERSEER_DATA_DIR:dir+'/data',OVERSEER_API_TOKEN:token,OVERSEER_DESKTOP_MANAGED:'1'},stdio:['pipe','ignore','ignore']});
const delay=ms=>new Promise(r=>setTimeout(r,ms));
const api=async(path,body)=>{const r=await fetch('http://127.0.0.1:18467'+path,{method:body?'POST':'GET',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},body:body?(typeof body === "string" ? body : JSON.stringify(body)):undefined});if(!r.ok)throw Error('API '+r.status);return r.text()};
let chrome,ws;
try{
 for(let n=0;n<100;n++){try{await api('/healthz');break}catch{if(n===99)throw Error('daemon not ready');await delay(100)}}
 const preflight = await fetch('http://127.0.0.1:18467/api/messages', {method:'OPTIONS',headers:{Origin:'http://127.0.0.1:4173','Access-Control-Request-Method':'GET','Access-Control-Request-Headers':'authorization'}});
 if(preflight.status!==204 || preflight.headers.get('access-control-allow-origin')!=='http://127.0.0.1:4173')throw Error('trusted preflight rejected');
 const blocked = await fetch('http://127.0.0.1:18467/api/messages', {method:'OPTIONS',headers:{Origin:'https://evil.example','Access-Control-Request-Method':'GET'}});
 if(blocked.status!==403)throw Error('untrusted origin accepted');
 const unauthorized = await fetch('http://127.0.0.1:18467/api/messages?conversation_id=1', {headers:{Origin:'http://127.0.0.1:4173'}});
 if(unauthorized.status!==401)throw Error('REST authentication bypassed');
 await api('/api/settings',{key:'desktop_setup_complete',value:'true'});
 const id=await api('/api/conversations',{title:'Desktop history acceptance'});
 await api('/api/messages',JSON.stringify({conversation_id:'__EXACT__',role:'user',content:'Saved desktop question',hide_evidence:false}).replace('"__EXACT__"',id.trim()));
 await api('/api/messages',JSON.stringify({conversation_id:'__EXACT__',role:'assistant',content:'Saved desktop answer',hide_evidence:false}).replace('"__EXACT__"',id.trim()));
 chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless','--no-first-run','--no-default-browser-check','--user-data-dir='+dir+'/chrome','--remote-debugging-port=9241','about:blank'],{stdio:'ignore'});
 let pages;for(let n=0;n<100;n++){try{pages=await(await fetch('http://127.0.0.1:9241/json')).json();break}catch{await delay(100)}}
 ws=new WebSocket(pages.find(p=>p.type==='page').webSocketDebuggerUrl);await new Promise(r=>ws.addEventListener('open',r,{once:true}));
 let seq=0;const pending=new Map();ws.addEventListener('message',e=>{const v=JSON.parse(e.data);if(v.id){pending.get(v.id)?.(v);pending.delete(v.id)}});
 const call=(method,params={})=>new Promise((resolve,reject)=>{const id=++seq;pending.set(id,v=>v.error?reject(v.error):resolve(v.result));ws.send(JSON.stringify({id,method,params}))});
 const evaluate=async expression=>{const v=await call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(v.exceptionDetails)throw Error(JSON.stringify(v.exceptionDetails));return v.result.value};
 await call('Page.enable');await call('Runtime.enable');await call('Network.enable');await call('Network.setCacheDisabled',{cacheDisabled:true});
 await call('Page.addScriptToEvaluateOnNewDocument',{source:`if(location.origin==='http://127.0.0.1:4173'){localStorage.setItem('overseer.wsUrl','ws://127.0.0.1:18467/api/ws/chat');localStorage.setItem('overseer.token',${JSON.stringify(token)});localStorage.setItem('overseer.conv',${JSON.stringify(id.trim())});}`});
 await call('Page.navigate',{url:'http://127.0.0.1:4173'});await delay(1600);
 const restored=await evaluate("document.body.innerText.includes('Saved desktop question') && document.body.innerText.includes('Saved desktop answer') && !document.body.innerText.includes('Could not restore')");if(!restored)throw Error(await evaluate('document.body.innerText'));
 await evaluate("document.querySelector('textarea').focus()");await call('Input.insertText',{text:'/memory list'});await delay(100);await evaluate("document.querySelector('button[aria-label=Send]').click()");await delay(1600);
 if(!await evaluate("document.body.innerText.includes('No saved memories.')"))throw Error('send after restore: '+await evaluate('document.body.innerText'));
 await call('Page.reload');await delay(1300);if(!await evaluate("document.body.innerText.includes('Saved desktop answer') && document.body.innerText.includes('No saved memories.')"))throw Error('reload history');
 for(const width of [320,440,1100]){await call('Emulation.setDeviceMetricsOverride',{width,height:800,deviceScaleFactor:1,mobile:false});await delay(80);if(!await evaluate('document.documentElement.scrollWidth<=innerWidth'))throw Error('overflow '+width)}
 daemon.stdin.end();await Promise.race([new Promise(r=>daemon.once('exit',r)),delay(5000).then(()=>{throw Error('backend pipe EOF did not shut down')})]);
 console.log('PASS: real authenticated history restore, send after restore, reload persistence, 320/440/1100 layout, trusted CORS preflight, untrusted-origin rejection, REST auth and graceful managed-pipe shutdown.');
 await call('Browser.close');
}finally{ws?.close();chrome?.kill();if(daemon.exitCode===null)daemon.kill('SIGKILL');await delay(500);await rm(dir,{recursive:true,force:true,maxRetries:10,retryDelay:200})}
