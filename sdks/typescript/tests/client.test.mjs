import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { OverseerClient, ApiError } from '../dist/index.js';
import { encodeJson, decodeJson } from '../dist/json.js';
const id = '9223372036854775806';

async function server(handler, run) {
  const http = createServer(handler);await new Promise(resolve=>http.listen(0,'127.0.0.1',resolve));
  try { await run(`http://127.0.0.1:${http.address().port}`); }
  finally { http.closeAllConnections();await new Promise(resolve=>http.close(resolve)); }
}
test('exact IDs round-trip over REST in bodies, query strings, paths, and arrays', async()=>{
  const requests=[];
  await server(async(req,res)=>{
    let body='';for await(const chunk of req)body+=chunk;
    requests.push({path:req.url,body,auth:req.headers.authorization});
    if(req.url==='/api/conversations'&&req.method==='POST')res.end(id);
    else if(req.url.startsWith('/api/conversations?'))res.end(`[{"id":${id},"title":"SDK","created_at":"2026-10-05T00:00:00"}]`);
    else if(req.url.startsWith('/api/messages?'))res.end('[]');
    else if(req.url==='/api/artifacts/by_ids')res.end('[]');
    else {res.writeHead(204);res.end();}
  },async(baseUrl)=>{
    const client=new OverseerClient({baseUrl,token:'test-token'});
    assert.equal(await client.conversations.create('SDK'),id);
    assert.equal((await client.conversations.list())[0].id,id);
    await client.conversations.updateTitle(id,'renamed');
    await client.messages.list(id);
    await client.credentials.delete(id);
    await client.artifacts.getByIds([id,7n]);
    assert.equal(requests[2].body,`{"id":${id},"title":"renamed"}`);
    assert.equal(requests[3].path,`/api/messages?conversation_id=${id}`);
    assert.equal(requests[4].path,`/v1/credentials/${id}`);
    assert.equal(requests[5].body,`[${id},7]`);
    assert.ok(requests.every(r=>r.auth==='Bearer test-token'));
    await assert.rejects(client.conversations.delete(Number(id)),RangeError);
    await assert.rejects(client.credentials.delete('1/../../settings'),TypeError);
  });
});
test('JSON parser preserves quoted content and only normalizes numeric IDs',()=>{
  assert.deepEqual(decodeJson(`{"id":7,"count":3,"text":"id: ${id}","fraction":1.5,"nested":{"run_id":${id}},"approval_id":"uuid"}`),{id:'7',count:3,text:`id: ${id}`,fraction:1.5,nested:{run_id:id},approval_id:'uuid'});
  assert.equal(encodeJson({id,text:`"id":"${id}"`,approval_id:'uuid'}),`{"id":${id},"text":"\\\"id\\\":\\\"${id}\\\"","approval_id":"uuid"}`);
  assert.throws(()=>encodeJson({id:'9223372036854775808'}),RangeError);
});
test('POST works without a manual origin prefix; errors and redirects remain visible',async()=>{
  await server((req,res)=>{if(req.url==='/api/orchestrate'){res.end('plain answer');return;}res.writeHead(req.url==='/redirect'?302:401,{Location:'https://example.invalid'});res.end('denied');},async(baseUrl)=>{
    const client=new OverseerClient({baseUrl});
    assert.equal(await client.orchestrate(id,'hello'),'plain answer');
    await assert.rejects(client.agents.list(),e=>e instanceof ApiError&&e.status===401&&e.body==='denied');
    await assert.rejects(client.request('GET','/redirect'));
    await assert.rejects(client.request('GET','https://example.invalid'),TypeError);
  });
});
test('timeout includes a response body stalled after headers',async()=>{
  await server((req,res)=>{res.writeHead(200,{'Content-Type':'application/json'});res.write('[');},async(baseUrl)=>{
    await assert.rejects(new OverseerClient({baseUrl,timeout:30}).conversations.list());
  });
});
class Socket extends EventTarget {
  readyState=0;sent=[];closed=false;
  constructor(){super();queueMicrotask(()=>{this.readyState=1;this.dispatchEvent(new Event('open'));});}
  send(value){this.sent.push(JSON.parse(value));if(this.sent.at(-1).message){queueMicrotask(()=>{
    this.emit({type:'session',conversation_id:id});this.emit({type:'token',text:'hello'});this.emit({type:'approval',id:'approval-1',tool:'read_file',kind:'read',summary:'read',reason:'review'});
  });}if(this.sent.at(-1).approve||this.sent.at(-1).reject)queueMicrotask(()=>this.emit({type:'done',response:'finished'}));}
  emit(value){this.dispatchEvent(new MessageEvent('message',{data:JSON.stringify(value)}));}
  close(){this.closed=true;this.readyState=3;}
}
for(const approved of [false,true])test(`stream auth, exact session ID, explicit ${approved?'approval':'default rejection'}, and closure`,async()=>{
 let socket;const client=new OverseerClient({token:'test',webSocketFactory:()=>socket=new Socket()});
 const events=[];for await(const event of client.chat.stream({message:'hello',conversationId:id},approved?{onApproval:()=>true}:{}))events.push(event);
 assert.equal(events.at(-1).response,'finished');assert.equal(events[0].conversation_id,id);
 assert.deepEqual(socket.sent[0],{token:'test'});assert.equal(socket.sent[1].conversation_id,id);
 assert.deepEqual(socket.sent[2],{[approved?'approve':'reject']:'approval-1'});assert.ok(socket.closed);
});
test('breaking a stream closes its socket',async()=>{
 let socket;const client=new OverseerClient({webSocketFactory:()=>socket=new Socket()});
 for await(const event of client.chat.stream({message:'hello'}))break;
 assert.ok(socket.closed);
});
test('abort interrupts a waiting stream',async()=>{
 let socket;const control=new AbortController();const client=new OverseerClient({webSocketFactory:()=>{socket=new Socket();socket.send=()=>{};return socket;}});
 const promise=client.chat.stream({message:'hello'},{signal:control.signal}).next();setTimeout(()=>control.abort(),10);
 await assert.rejects(promise,e=>e.name==='AbortError');assert.ok(socket.closed);
});

test('abort interrupts a pending human approval callback', async()=>{
 let socket;const control=new AbortController();const client=new OverseerClient({webSocketFactory:()=>socket=new Socket()});
 const run=(async()=>{for await(const event of client.chat.stream({message:'hello'},{signal:control.signal,onApproval:()=>{setTimeout(()=>control.abort(),10);return new Promise(()=>{});}})) {}})();
 await assert.rejects(run,e=>e.name==='AbortError');assert.ok(socket.closed);
});
test('CommonJS build is usable by editor integrations', async()=>{
 const {createRequire}=await import('node:module');const require=createRequire(import.meta.url);
 assert.equal(new (require('../dist-cjs/index.js').OverseerClient)().token,undefined);
});
