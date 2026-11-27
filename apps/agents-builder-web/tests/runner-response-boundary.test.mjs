import test from 'node:test';
import assert from 'node:assert/strict';
import {LocalRunnerClient} from '../runner-client.mjs';
const client = response => new LocalRunnerClient({token:'test-runner-secret',origin:'http://127.0.0.1:8765',fetchImpl:async()=>response});
const file = () => new File([new Uint8Array(24)],'model.gguf');
const json = raw => new Response(raw,{headers:{'content-type':'application/json'}});

test('rejects duplicate keys and escaped aliases in inference response',async()=>{
  await assert.rejects(client(json('{"text":"first","te\\u0078t":"second"}')).chat('x'),/invalid JSON response/);
});
test('rejects invalid UTF-8 instead of silently replacing bytes',async()=>{
  const bytes=new Uint8Array([123,34,116,101,120,116,34,58,34,255,34,125]);
  await assert.rejects(client(json(bytes)).chat('x'),/invalid JSON response/);
});
test('cancels oversized stream before consuming its remaining chunks',async()=>{
  let canceled=false, pulls=0;
  const stream=new ReadableStream({pull(c){pulls++;if(pulls===4)c.close();else c.enqueue(new Uint8Array(300*1024));},cancel(){canceled=true;}},{highWaterMark:0});
  await assert.rejects(client(json(stream)).chat('x'),/response limit/);
  assert.equal(canceled,true);assert.equal(pulls,2);
});
test('upload receipt rejects null and duplicate keys',async()=>{
  await assert.rejects(client(json('null')).importGguf(file()),/invalid upload response/);
  const raw='{"name":"other.gguf","name":"model.gguf","size_bytes":24,"sha256":"'+'a'.repeat(64)+'","version":3}';
  await assert.rejects(client(json(raw)).importGguf(file()),/invalid upload response/);
});
test('upload receipt enforces byte length and cancels an excessive stream',async()=>{
  let canceled=false;
  const stream=new ReadableStream({start(c){c.enqueue(new Uint8Array(8193));},cancel(){canceled=true;}});
  // A finite stream for RED, so the former text() path does not hang.
  const finite=new ReadableStream({start(c){c.enqueue(new TextEncoder().encode('"'+'ą'.repeat(5000)+'"'));c.close();}});
  await assert.rejects(client(json(finite)).importGguf(file()),/upload response limit/);
  await assert.rejects(client(json(stream)).importGguf(file()),/upload response limit/);
  assert.equal(canceled,true);
});
test('valid inference and import receipt remain accepted',async()=>{
  assert.equal((await client(json('{"text":"Zażółć"}')).chat('x')).text,'Zażółć');
  const receipt={name:'model.gguf',size_bytes:24,sha256:'a'.repeat(64),version:3};
  assert.deepEqual(await client(json(JSON.stringify(receipt))).importGguf(file()),receipt);
});
