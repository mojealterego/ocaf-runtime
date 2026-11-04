import test from 'node:test';
import assert from 'node:assert/strict';
import {LocalRunnerClient, LocalRunnerError} from '../runner-client.mjs';
import {defaultSettings} from '../models.mjs';

test('local runner refuses non-loopback origins and invalid session tokens', () => {
  assert.throws(()=>new LocalRunnerClient({token:'valid-test-secret',origin:'https://external.example'}),/loopback/);
  assert.throws(()=>new LocalRunnerClient({token:'bad\nvalue',origin:'http://127.0.0.1:8765'}),/token/);
  assert.throws(()=>new LocalRunnerClient({token:'',origin:'http://localhost:8765'}),/token/);
});
test('local runner uses only authenticated same-origin fixed paths without cookies',async()=>{
  const calls=[];
  const fake=async(url,init)=>{
    calls.push([url,init]);
    if(url.endsWith('/models'))return new Response(JSON.stringify({models:[{name:'safe.Q4_K_M.gguf',size_bytes:64,version:3,tensor_count:1}]}),{status:200});
    if(url.endsWith('/status'))return new Response(JSON.stringify({state:'ready',model:'safe.Q4_K_M.gguf'}),{status:200});
    if(url.endsWith('/start'))return new Response(JSON.stringify({state:'loading',model:'safe.Q4_K_M.gguf'}),{status:200});
    if(url.endsWith('/chat'))return new Response(JSON.stringify({text:'OCAF output',model:'safe.Q4_K_M.gguf'}),{status:200});
    return new Response(JSON.stringify({state:'stopped',model:null}),{status:200});
  };
  const client=new LocalRunnerClient({token:'valid-test-secret',origin:'http://127.0.0.1:8765',fetchImpl:fake});
  assert.equal((await client.models()).length,1);
  assert.equal((await client.status()).state,'ready');
  await client.start('safe.Q4_K_M.gguf',defaultSettings());
  assert.equal((await client.chat('Witaj!')).text,'OCAF output');
  await client.stop();
  assert.deepEqual(calls.map(x=>x[0]),[
    'http://127.0.0.1:8765/v1/local-runner/models',
    'http://127.0.0.1:8765/v1/local-runner/status',
    'http://127.0.0.1:8765/v1/local-runner/start',
    'http://127.0.0.1:8765/v1/local-runner/chat',
    'http://127.0.0.1:8765/v1/local-runner/stop'
  ]);
  for(const [,opts] of calls){
    assert.equal(opts.credentials,'omit');
    assert.equal(opts.redirect,'error');
    assert.equal(opts.headers.Authorization,'Bearer valid-test-secret');
  }
  assert.equal(JSON.stringify(calls).includes('api_key'),false);
  assert.equal(JSON.stringify(calls).includes('provider'),false);
  client.disconnect();
  await assert.rejects(client.models(),/disconnected/);
});
test('runner bounds prompt, filenames and denies malformed responses',async()=>{
  const client=new LocalRunnerClient({token:'test-key-abc',origin:'http://localhost:8765',
    fetchImpl:async()=>new Response(JSON.stringify({models:[]}),{status:200})});
  await assert.rejects(client.start('../secrets.gguf',defaultSettings()),/filename/);
  await assert.rejects(client.chat(''),/prompt/);
  await assert.rejects(client.chat('a'.repeat(12001)),/prompt/);
  await assert.rejects(client.status(),/response/);
});
test('runner preserves structured, bounded HTTP error without leaking secrets',async()=>{
  const client=new LocalRunnerClient({token:'test-super-secret',origin:'http://127.0.0.1:8765',
    fetchImpl:async()=>new Response(JSON.stringify({error:'model is loading'}),{status:409})});
  await assert.rejects(client.chat('Hello'),error=>{
    assert.ok(error instanceof LocalRunnerError);
    assert.equal(error.status,409);
    assert.ok(!error.message.includes('test-super-secret'));
    return true;
  });
});
