'use strict';
const assert=require('node:assert/strict'),{test}=require('node:test');
const graph=require('../canonical-graph.js').OCAFCanonicalGraph;
const create=require('../cloud.js').OCAFKaiosCloud.createClient;
const p={schema_version:1,tenant_id:'tenant',project_id:'agent',revision:1,max_cost_minor:0,grants:[],
  nodes:[{id:'input',kind:'input',inputs:[],outputs:[{name:'out',kind:'text'}],tool_name:null,max_cost_minor:0},{id:'output',kind:'output',inputs:[{name:'in',kind:'text'}],outputs:[],tool_name:null,max_cost_minor:0}],
  edges:[{from_node:'input',from_port:'out',to_node:'output',to_port:'in'}]};
const h={revision:1,sha256:'a'.repeat(64)};
function response(value){const b=new TextEncoder().encode(JSON.stringify(value));return {ok:true,status:200,url:'',headers:{get:k=>({'Content-Type':'application/json','Content-Length':String(b.length)}[k]??null)},arrayBuffer:async()=>b.buffer};}
function env(reply){const calls=[];return {calls,location:{origin:'https://builder.example.test',protocol:'https:'},TextDecoder,AbortController,setTimeout,clearTimeout,fetch:async(url,options)=>{calls.push({url,options});return reply(url,options);}};}
test('HTTPS load, credentials omit, redirect error',async()=>{const e=env(()=>response({project:p,head:h}));assert.deepEqual(await create(e,'tenant','token',graph).load('agent'),{project:p,head:h});assert.equal(e.calls[0].url,'https://builder.example.test/v1/projects/agent');assert.equal(e.calls[0].options.headers.Authorization,'Bearer token');assert.equal(e.calls[0].options.credentials,'omit');assert.equal(e.calls[0].options.redirect,'error');});
test('insecure or app origin and invalid token denied',()=>{const e=env(()=>{throw Error('network');});for(const origin of ['http://localhost','app://app','https://host/path','https://user@host'])assert.throws(()=>create({...e,location:{origin,protocol:origin.split(':')[0]+':'}},'tenant','token',graph),/HTTPS/);assert.throws(()=>create(e,'tenant','bad token',graph),/token/);assert.equal(e.calls.length,0);});
test('cross tenant and mismatched receipt',async()=>{await assert.rejects(create(env(()=>response({project:{...p,tenant_id:'other'},head:h})),'tenant','token',graph).load('agent'),/identity/);await assert.rejects(create(env(()=>response({project:p,head:{...h,revision:2}})),'tenant','token',graph).load('agent'),/revision/);});
test('CAS rejects gaps and 409, no retry',async()=>{const e=env(()=>({ok:false,status:409,url:''})),c=create(e,'tenant','token',graph);assert.throws(()=>c.save({...p,revision:3},h),/CAS/);await assert.rejects(c.save({...p,revision:2},h),/409/);assert.equal(e.calls.length,1);assert.deepEqual(JSON.parse(e.calls[0].options.body).expected,h);});
test('preview matches canonical plan',async()=>{const plan={...graph.compile(p,'tenant'),project_sha256:'b'.repeat(64)};assert.deepEqual(await create(env(()=>response(plan)),'tenant','token',graph).preview(p),plan);await assert.rejects(create(env(()=>response({...plan,ordered_nodes:['output','input']})),'tenant','token',graph).preview(p),/plan mismatch/);});
test('bounded response and redirect denial',async()=>{for(const headers of [{'Content-Length':'999999'},{'Content-Length':null},{'Content-Encoding':'gzip'},{'Content-Type':'text/html'}]){const r=response({project:p,head:h}),get=r.headers.get;r.headers.get=k=>Object.hasOwn(headers,k)?headers[k]:get(k);await assert.rejects(create(env(()=>r),'tenant','token',graph).load('agent'),/limit|bounded|type/);}await assert.rejects(create(env(()=>({...response({project:p,head:h}),redirected:true})),'tenant','token',graph).load('agent'),/redirect/);});
test('concurrent and disconnected requests rejected',async()=>{let done;const e=env(()=>new Promise(resolve=>{done=resolve;})),c=create(e,'tenant','token',graph),pending=c.load('agent');await Promise.resolve();await assert.rejects(c.load('agent'),/in progress/);c.disconnect();done(response({project:p,head:h}));await assert.rejects(pending,/disconnected/);await assert.rejects(c.load('agent'),/disconnected/);});
test('disconnect before scheduling prevents credential send',async()=>{const e=env(()=>{throw Error('network');}),c=create(e,'tenant','token',graph),pending=c.load('agent');c.disconnect();await assert.rejects(pending,/disconnected/);assert.equal(e.calls.length,0);});
test('oversized stream cancelled',async()=>{let cancelled=false;const r=response({project:p,head:h}),get=r.headers.get;r.headers.get=k=>k==='Content-Length'?null:get(k);r.body={getReader:()=>({read:async()=>({done:false,value:new Uint8Array(131073)}),cancel:async()=>{cancelled=true;}})};await assert.rejects(create(env(()=>r),'tenant','token',graph).load('agent'),/limit/);assert.equal(cancelled,true);});
test('timeout revokes response without AbortController',async()=>{let fire,reply;const e=env(()=>new Promise(resolve=>{reply=resolve;}));e.AbortController=undefined;e.setTimeout=f=>{fire=f;return 1;};e.clearTimeout=()=>{};const c=create(e,'tenant','token',graph),pending=c.load('agent');await Promise.resolve();fire();await assert.rejects(pending,/timeout/);reply(response({project:p,head:h}));});

test('timed out CAS revokes client even when legacy fetch cannot abort',async()=>{
  let fire,late;
  const e=env(()=>e.calls.length===1?new Promise(resolve=>{late=resolve;}):response({project:p,head:h}));
  e.AbortController=undefined;e.setTimeout=f=>{fire=f;return 1;};e.clearTimeout=()=>{};
  const c=create(e,'tenant','token',graph),pending=c.save(p,null);
  await Promise.resolve();fire();await assert.rejects(pending,/timeout/);
  await assert.rejects(c.load('agent'),/disconnected/);assert.equal(e.calls.length,1);
  late(response(h));
});
test('disconnect settles stalled fetch without waiting for timer or network',async()=>{
  const e=env(()=>new Promise(()=>{}));e.AbortController=undefined;
  let cleared=false;e.setTimeout=()=>1;e.clearTimeout=()=>{cleared=true;};
  const c=create(e,'tenant','token',graph),pending=c.load('agent');await Promise.resolve();
  c.disconnect();
  const result=await Promise.race([pending.then(()=> 'success',err=>err.message),new Promise(resolve=>setImmediate(()=>resolve('still pending')))]);
  assert.match(result,/disconnected/);assert.equal(cleared,true);assert.equal(e.calls.length,1);
});
test('timeout cancels stalled streaming reader without AbortController',async()=>{
  let fire,started,cancelled=0;const ready=new Promise(resolve=>{started=resolve;});
  const r=response({project:p,head:h});r.body={getReader:()=>({read:()=>{started();return new Promise(()=>{});},cancel:async()=>{cancelled++;}})};
  const e=env(()=>r);e.AbortController=undefined;e.setTimeout=f=>{fire=f;return 1;};e.clearTimeout=()=>{};
  const c=create(e,'tenant','token',graph),pending=c.load('agent');await ready;fire();
  await assert.rejects(pending,/timeout/);assert.equal(cancelled,1);
});
