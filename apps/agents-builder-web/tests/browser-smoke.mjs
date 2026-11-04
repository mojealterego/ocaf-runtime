/**
 * Browser E2E smoke without third-party JS dependencies.
 * Requires Chrome/Chromium and Node 22, serves the real static editor on loopback.
 * Exercises DOM interaction, Code↔NoCode, persistence and invalid DSL denial.
 */
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {mkdtemp, readFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

const ROOT = new URL('../', import.meta.url);
const ALLOWED = new Set(['/index.html','/app.mjs','/editor.mjs','/cloud.mjs','/i18n.mjs','/credentials.mjs','/models.mjs','/integrations-ui.mjs','/runner-client.mjs','/style.css','/manifest.webmanifest','/sw.js']);
const MIME = {'.html':'text/html','.mjs':'text/javascript','.css':'text/css','.webmanifest':'application/manifest+json','.js':'text/javascript'};
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(fn, description, maxMs = 20000) {
  const end = Date.now() + maxMs;
  let lastError;
  while (Date.now() < end) {
    try { if (await fn()) return; } catch (error) { lastError = error; }
    await pause(100);
  }
  throw new Error('Timed out: ' + description + (lastError ? ' (' + lastError.message + ')' : ''));
}

const cloudProjects = new Map();
let delayedCloudPut = false;
let cloudPutReceived = false;
const runnerModels = [{name:'sample.gguf',size_bytes:1024,version:3,tensor_count:1}];
const server = createServer(async (request, response) => {
  try {
    const pathname = new URL(request.url, 'http://localhost').pathname;
    const file = pathname === '/' ? '/index.html' : pathname;
    if (file.startsWith('/v1/local-runner/')) {
      const send=(status,data)=>{
        response.writeHead(status,{'Content-Type':'application/json','Cache-Control':'no-store'});
        response.end(JSON.stringify(data));
      };
      if(request.headers.authorization!=='Bearer runner-test-secret'){
        send(401,{error:'local token required'});return;
      }
      if(file==='/v1/local-runner/models'&&request.method==='GET'){
        send(200,{models:runnerModels});return;
      }
      if(file==='/v1/local-runner/status'&&request.method==='GET'){
        send(200,{state:'ready',model:'sample.gguf'});return;
      }
      const payload=await new Promise((resolve,reject)=>{
        let body='';
        request.on('data',chunk=>{body+=chunk;if(body.length>32768)reject(new Error('limit'));});
        request.on('end',()=>resolve(body));
        request.on('error',reject);
      });
      if(file==='/v1/local-runner/import'&&request.method==='POST'){
        const name=request.headers['x-ocaf-model-name'];
        if(name!=='new-import.gguf'||request.headers['content-type']!=='application/octet-stream'||
           payload.length!==24){send(422,{error:'bad GGUF upload'});return;}
        runnerModels.push({name,size_bytes:24,version:3,tensor_count:1});
        send(201,{name,size_bytes:24,version:3,
          sha256:createHash('sha256').update(payload).digest('hex')});
        return;
      }
      if(file==='/v1/local-runner/start'&&request.method==='POST'){
        const data=JSON.parse(payload);
        if(data.model!=='sample.gguf'||data.settings.context_size!==8192){
          send(422,{error:'invalid model/settings'});return;
        }
        send(200,{state:'loading',model:data.model});return;
      }
      if(file==='/v1/local-runner/chat'&&request.method==='POST'){
        const data=JSON.parse(payload);
        send(200,{text:'MODEL E2E: '+data.prompt,model:'sample.gguf'});return;
      }
      if(file==='/v1/local-runner/stop'&&request.method==='POST'){
        send(200,{state:'stopped',model:null});return;
      }
      send(404,{error:'unknown local runner endpoint'});return;
    }
    if (file.startsWith('/v1/projects/')) {
      const send = (status, value) => {
        response.writeHead(status, {'Content-Type':'application/json','Cache-Control':'no-store'});
        response.end(JSON.stringify(value));
      };
      if (request.headers.authorization !== 'Bearer test-token') { send(403,{error:'http_403'}); return; }
      const payload = request.method === 'GET' ? '' : await new Promise((resolve,reject) => {
        let body = '';
        request.on('data', chunk => { body += chunk; if (body.length > 128*1024) reject(new Error('limit')); });
        request.on('end', () => resolve(body));
        request.on('error', reject);
      });
      if (file === '/v1/projects/preview' && request.method === 'POST') {
        const project = JSON.parse(payload);
        send(200,{ordered_nodes:project.nodes.map(node=>node.id),requested_tools:[],max_cost_minor:project.max_cost_minor,project_sha256:createHash('sha256').update(payload).digest('hex')});
        return;
      }
      const id = file.slice('/v1/projects/'.length);
      if (!/^[A-Za-z0-9_.-]{1,64}$/.test(id)) {send(404,{error:'http_404'});return;}
      if (request.method === 'GET') {
        const record = cloudProjects.get(id);
        send(record ? 200 : 404,record ?? {error:'http_404'});
        return;
      }
      if (request.method === 'PUT') {
        if (delayedCloudPut) {
          cloudPutReceived = true;
          await pause(600);
          delayedCloudPut = false;
        }
        const {project,expected} = JSON.parse(payload);
        const previous = cloudProjects.get(id);
        if (project.tenant_id !== 'tenant-a' || project.project_id !== id) {send(403,{error:'http_403'});return;}
        if (JSON.stringify(previous?.head ?? null) !== JSON.stringify(expected) ||
            project.revision !== (previous?.head.revision ?? 0) + 1) {
          send(409,{error:'http_409'});return;
        }
        const head = {revision:project.revision,sha256:createHash('sha256').update(JSON.stringify(project)).digest('hex')};
        cloudProjects.set(id,{project,head});
        send(200,head);
        return;
      }
      send(405,{error:'http_405'});return;
    }
    if (request.method !== 'GET' || !ALLOWED.has(file)) {
      response.writeHead(404); response.end('Not found'); return;
    }
    const bytes = await readFile(new URL('.' + file, ROOT));
    const ext = file.slice(file.lastIndexOf('.'));
    response.writeHead(200, {'Content-Type': MIME[ext] || 'application/octet-stream', 'Cache-Control':'no-store'});
    response.end(bytes);
  } catch {
    response.writeHead(500); response.end('Server error');
  }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
const tmp = await mkdtemp(join(tmpdir(), 'ocaf-builder-chrome-'));
const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
  '--headless=new', '--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu',
  '--disable-background-networking', '--no-first-run', '--no-default-browser-check',
  '--remote-allow-origins=*', '--remote-debugging-port=0', '--user-data-dir=' + tmp,
  'about:blank'
], {stdio:'ignore'});
let ws;
try {
  const debugFile = join(tmp, 'DevToolsActivePort');
  let debugPort;
  await until(async () => {
    if (chrome.exitCode !== null) throw new Error('Chrome exited: ' + chrome.exitCode);
    try { debugPort = Number((await readFile(debugFile, 'utf8')).split('\n')[0]); return debugPort > 0; }
    catch { return false; }
  }, 'Chrome DevTools port');
  const targets = await (await fetch('http://127.0.0.1:' + debugPort + '/json/list')).json();
  const page = targets.find(target => target.type === 'page');
  assert.ok(page, 'Chrome has a page target');
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, {once:true});
    ws.addEventListener('error', reject, {once:true});
  });
  let sequence = 0;
  const pending = new Map(), exceptions = [];
  ws.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    if (message.method === 'Runtime.exceptionThrown') exceptions.push(message.params.exceptionDetails.text);
    if (!message.id) return;
    const task = pending.get(message.id);
    if (!task) return;
    pending.delete(message.id);
    if (message.error) task.reject(new Error(message.error.message));
    else task.resolve(message.result);
  });
  const cmd = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, {resolve,reject});
    ws.send(JSON.stringify({id,method,params}));
  });
  const evaluate = async expression => {
    const result = await cmd('Runtime.evaluate', {expression,returnByValue:true,awaitPromise:true});
    if (result.exceptionDetails) throw new Error('Browser evaluation failed: ' + result.exceptionDetails.text);
    return result.result.value;
  };
  await cmd('Runtime.enable');
  await cmd('Page.enable');
  await cmd('Page.navigate', {url:'http://127.0.0.1:' + port + '/'});
  await until(async () => await evaluate('document.querySelectorAll(".node-card").length') === 2, 'NoCode initial render');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'), 'r1');

  await evaluate('document.getElementById("node-id").value="step"; document.getElementById("add-transform").click()');
  await until(async () => await evaluate('document.querySelectorAll(".node-card").length') === 3, 'NoCode insert');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'), 'r2');

  await evaluate('document.getElementById("mode-code").click()');
  assert.equal(await evaluate('document.getElementById("code-view").hidden'), false);
  await evaluate('(()=>{const e=document.getElementById("dsl");const p=JSON.parse(e.value);p.max_cost_minor=250;e.value=JSON.stringify(p);e.dispatchEvent(new Event("input",{bubbles:true}));document.getElementById("apply-code").click();})()');
  await until(async () => await evaluate('document.getElementById("revision-label").textContent') === 'r3', 'Code apply');

  await evaluate('(()=>{const e=document.getElementById("dsl");const p=JSON.parse(e.value);p.untrusted_field=true;e.value=JSON.stringify(p);e.dispatchEvent(new Event("input",{bubbles:true}));document.getElementById("apply-code").click();})()');
  assert.match(await evaluate('document.getElementById("message").textContent'), /schema/i);
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'), 'r3');

  // Invalid unapplied Code text must not silently validate or export the last saved graph.
  await evaluate('document.getElementById("preview").click()');
  assert.match(await evaluate('document.getElementById("message").textContent'), /unapplied Code DSL/i);
  await evaluate('document.getElementById("export").click()');
  assert.match(await evaluate('document.getElementById("message").textContent'), /unapplied Code DSL/i);

  await cmd('Page.reload', {ignoreCache:true});
  await until(async () => await evaluate('document.querySelectorAll(".node-card").length') === 3, 'offline persistence after reload');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'), 'r3');
  await evaluate('document.getElementById("preview").click()');
  assert.match(await evaluate('document.getElementById("preview-output").textContent'), /no execution/i);
  assert.match(await evaluate('document.getElementById("message").textContent'), /validated locally/i);
  assert.equal(await evaluate('document.documentElement.lang'), 'pl-PL');
  await evaluate('document.getElementById("ui-language").value="cs-CZ";document.getElementById("ui-language").dispatchEvent(new Event("change"))');
  assert.equal(await evaluate('document.querySelector("h1").textContent'), 'Navrhujte agenty.');
  await evaluate('document.getElementById("ui-language").value="ar-SA";document.getElementById("ui-language").dispatchEvent(new Event("change"))');
  assert.equal(await evaluate('document.documentElement.dir'), 'rtl');
  await evaluate('document.getElementById("ui-language").value="pl-PL";document.getElementById("ui-language").dispatchEvent(new Event("change"))');
  assert.equal(await evaluate('document.documentElement.dir'), 'ltr');
  await evaluate('document.getElementById("api-provider").value="openai";document.getElementById("api-secret").value="sk-super-secret-test";document.getElementById("api-add").click()');
  assert.equal(await evaluate('document.getElementById("api-secret").value'), '');
  assert.equal(await evaluate('JSON.stringify(localStorage).includes("sk-super-secret-test")'),false);

  // Actual binary GGUF signature and version from a browser File object.
  await evaluate('(()=>{const e=document.getElementById("model-kind");e.value="file";e.dispatchEvent(new Event("change"));const b=new ArrayBuffer(24);const v=new DataView(b);["G","G","U","F"].forEach((c,i)=>v.setUint8(i,c.charCodeAt(0)));v.setUint32(4,3,true);v.setBigUint64(8,1n,true);const t=new DataTransfer();t.items.add(new File([b],"sample.gguf",{type:"application/octet-stream"}));const f=document.getElementById("gguf-file");f.files=t.files;f.dispatchEvent(new Event("change",{bubbles:true}));})()');
  await until(async () => await evaluate('document.getElementById("gguf-status").textContent.includes("GGUF v3")'), 'GGUF binary inspection');
  await evaluate('document.getElementById("model-setting-context_size").value="8192";document.getElementById("model-save").click()');
  await until(async () => await evaluate('document.getElementById("model-status").textContent.includes("Profil zapisany")'), 'model profile persisted');
  assert.equal(await evaluate('JSON.stringify(localStorage).includes("sk-super-secret-test")'),false);
  assert.equal(await evaluate('JSON.stringify(localStorage).includes("sample.gguf")'),true);
  await evaluate('document.getElementById("api-remove").click()');
  assert.match(await evaluate('document.getElementById("api-status").textContent'), /Brak klucza/);

  await evaluate('document.getElementById("runner-token").value="runner-test-secret";document.getElementById("runner-connect").click()');
  await until(async()=>await evaluate('document.getElementById("runner-model").value')==='sample.gguf','runner connection and model catalog');
  await evaluate('(()=>{const b=new ArrayBuffer(24);const v=new DataView(b);["G","G","U","F"].forEach((c,i)=>v.setUint8(i,c.charCodeAt(0)));v.setUint32(4,3,true);v.setBigUint64(8,1n,true);const d=new DataTransfer();d.items.add(new File([b],"new-import.gguf",{type:"application/octet-stream"}));const f=document.getElementById("runner-file");f.files=d.files;f.dispatchEvent(new Event("change",{bubbles:true}));document.getElementById("runner-import").click();})()');
  await until(async()=>await evaluate('document.getElementById("runner-status").textContent.includes("Zaimportowano")'),'GGUF import to loopback runner');
  assert.ok(runnerModels.some(m=>m.name==='new-import.gguf'),'model uploaded through authenticated browser request');
  await evaluate('document.getElementById("runner-model").value="sample.gguf";document.getElementById("runner-start").click()');
  await until(async()=>await evaluate('document.getElementById("runner-status").textContent.includes("loading")'),'runner start with selected settings');
  await evaluate('document.getElementById("runner-prompt").value="Dzień dobry!";document.getElementById("runner-send").click()');
  await until(async()=>await evaluate('document.getElementById("runner-answer").textContent.includes("MODEL E2E: Dzień dobry!")'),'local inference request');
  await evaluate('document.getElementById("runner-stop").click()');
  await until(async()=>await evaluate('document.getElementById("runner-status").textContent.includes("stopped")'),'runner stop');
  assert.equal(await evaluate('JSON.stringify(localStorage).includes("runner-test-secret")'),false,'local token not persisted');
  await evaluate('document.getElementById("workspace-cloud").click()');
  assert.equal(await evaluate('document.getElementById("cloud-controls").hidden'), false);
  assert.equal(await evaluate('document.getElementById("cloud-save").disabled'), true);
  await evaluate('document.getElementById("cloud-tenant").value="tenant-a";document.getElementById("cloud-token").value="test-token";document.getElementById("cloud-connect").click()');
  assert.equal(await evaluate('document.getElementById("cloud-token").value'), '', 'token cleared from input');
  await evaluate('document.getElementById("project-id").value="cloud-demo";document.getElementById("cloud-new").click()');
  await until(async () => {
    if (await evaluate('document.getElementById("cloud-save").disabled') === false) return true;
    const state = await evaluate('JSON.stringify({message:document.getElementById("message").textContent,cloud:document.getElementById("cloud-status").textContent,newDisabled:document.getElementById("cloud-new").disabled,project:document.getElementById("project-title").textContent})');
    throw new Error(state);
  }, 'new cloud draft');
  assert.equal(await evaluate('document.querySelectorAll(".node-card").length'), 2, 'cloud graph renders under authenticated tenant');
  await evaluate('document.getElementById("cloud-save").click()');
  await until(async () => await evaluate('document.getElementById("cloud-save").disabled') === true, 'first CAS cloud save');
  assert.equal(cloudProjects.get('cloud-demo').head.revision,1);
  await evaluate('document.getElementById("mode-nocode").click();document.getElementById("node-id").value="remote-step";document.getElementById("add-transform").click()');
  await until(async () => await evaluate('document.getElementById("cloud-save").disabled') === false, 'cloud edit pending');
  assert.equal(await evaluate('document.querySelectorAll(".node-card").length'), 3, 'cloud graph reflects NoCode edit');
  // Unapplied Code DSL must not be silently omitted from a successful Cloud save.
  await evaluate('document.getElementById("mode-code").click()');
  await evaluate('(()=>{const e=document.getElementById("dsl");const p=JSON.parse(e.value);p.max_cost_minor=4321;e.value=JSON.stringify(p);e.dispatchEvent(new Event("input",{bubbles:true}));})()');
  await evaluate('document.getElementById("cloud-save").click()');
  assert.match(await evaluate('document.getElementById("message").textContent'), /unapplied Code DSL/i);
  assert.equal(cloudProjects.get('cloud-demo').head.revision,1,'dirty Code must not trigger Cloud CAS');
  await evaluate('document.getElementById("apply-code").click()');
  await evaluate('document.getElementById("cloud-save").click()');
  await until(async () => await evaluate('document.getElementById("cloud-save").disabled') === true, 'second CAS cloud save');
  assert.equal(cloudProjects.get('cloud-demo').head.revision,2);
  // Edit the same Cloud project after PUT starts but before the server responds.
  // The successful older CAS must not clear the newer unsaved draft.
  await evaluate('document.getElementById("node-id").value="inflight-a";document.getElementById("add-transform").click()');
  await until(async () => await evaluate('document.getElementById("cloud-save").disabled') === false, 'third cloud draft');
  delayedCloudPut = true;
  cloudPutReceived = false;
  await evaluate('document.getElementById("cloud-save").click()');
  // A second submit while the first CAS is in flight must be rejected client-side.
  await evaluate('document.getElementById("cloud-save").click()');
  assert.equal(await evaluate('document.getElementById("cloud-save").disabled'), true,
               'concurrent Cloud save must disable duplicate submission');
  await until(() => cloudPutReceived, 'delayed Cloud PUT received');
  // Workspace changes during a CAS request would orphan the server's new head.
  await evaluate('document.getElementById("workspace-local").click()');
  assert.equal(await evaluate('document.getElementById("workspace-cloud").getAttribute("aria-pressed")'), 'true',
               'Cloud workspace cannot be switched while CAS is in flight');
  assert.equal(await evaluate('document.getElementById("workspace-local").disabled'), true,
               'workspace switch disabled during Cloud save');

  await evaluate('document.getElementById("node-id").value="inflight-b";document.getElementById("add-transform").click()');
  await until(async () => await evaluate('document.getElementById("cloud-status").textContent.includes("niezapisany szkic")'), 'newer draft pending');
  await until(async () => cloudProjects.get('cloud-demo').head.revision === 3, 'older CAS completes');
  await until(async () => await evaluate('document.getElementById("revision-label").textContent') === 'r4 (szkic)', 'newer draft preserved after CAS');
  assert.equal(await evaluate('document.querySelectorAll(".node-card").length'), 5);
  assert.equal(cloudProjects.get('cloud-demo').project.nodes.length, 4, 'older save did not include concurrent edit');
  await evaluate('document.getElementById("cloud-save").click()');
  await until(async () => cloudProjects.get('cloud-demo').head.revision === 4, 'newer draft saved');
  assert.equal(cloudProjects.get('cloud-demo').project.nodes.length, 5, 'newer edit persisted on retry');

  await evaluate('document.getElementById("workspace-local").click()');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'),'r3');
  await evaluate('document.getElementById("workspace-cloud").click()');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'),'r4');
  await evaluate('document.getElementById("cloud-open").click()');
  await until(async () => await evaluate('document.getElementById("project-title").textContent') === 'cloud-demo', 'authenticated cloud reopen');
  assert.deepEqual(exceptions, [], 'no uncaught browser exceptions');
  process.stdout.write('PASS: Chrome Code/NoCode GUI, local persistence, authenticated Cloud transport mock, CAS and workspace switch\n');
} finally {
  if (ws) ws.close();
  chrome.kill('SIGKILL');
  await new Promise(resolve => server.close(resolve));
  await rm(tmp, {recursive:true,force:true});
}
