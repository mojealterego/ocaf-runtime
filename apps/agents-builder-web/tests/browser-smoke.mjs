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
const ALLOWED = new Set(['/index.html','/app.mjs','/editor.mjs','/cloud.mjs','/style.css','/manifest.webmanifest','/sw.js']);
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
const server = createServer(async (request, response) => {
  try {
    const pathname = new URL(request.url, 'http://localhost').pathname;
    const file = pathname === '/' ? '/index.html' : pathname;
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

  await evaluate('document.getElementById("preview").click()');
  assert.match(await evaluate('document.getElementById("preview-output").textContent'), /no execution/i);

  await cmd('Page.reload', {ignoreCache:true});
  await until(async () => await evaluate('document.querySelectorAll(".node-card").length') === 3, 'offline persistence after reload');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'), 'r3');
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
  await evaluate('document.getElementById("cloud-save").click()');
  await until(async () => await evaluate('document.getElementById("cloud-save").disabled') === true, 'first CAS cloud save');
  assert.equal(cloudProjects.get('cloud-demo').head.revision,1);
  await evaluate('document.getElementById("mode-nocode").click();document.getElementById("node-id").value="remote-step";document.getElementById("add-transform").click()');
  await until(async () => await evaluate('document.getElementById("cloud-save").disabled') === false, 'cloud edit pending');
  await evaluate('document.getElementById("cloud-save").click()');
  await until(async () => await evaluate('document.getElementById("cloud-save").disabled') === true, 'second CAS cloud save');
  assert.equal(cloudProjects.get('cloud-demo').head.revision,2);
  await evaluate('document.getElementById("workspace-local").click()');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'),'r3');
  await evaluate('document.getElementById("workspace-cloud").click()');
  assert.equal(await evaluate('document.getElementById("revision-label").textContent'),'r2');
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
