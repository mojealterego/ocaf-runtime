import test from 'node:test';
import assert from 'node:assert/strict';
import {CloudWorkspaceClient, CloudApiError} from '../cloud.mjs';
import {newProject} from '../editor.mjs';

const head = {revision:1,sha256:'a'.repeat(64)};
const project = newProject('tenant-a','hello');
const clientWith = (reply, requests) => new CloudWorkspaceClient({
  token:'opaque-access-token', origin:'https://builder.example.test',
  fetchImpl:async (url,options) => { requests.push({url,options}); return reply(url,options); }
});
const json = (value,status=200) => new Response(JSON.stringify(value), {status,headers:{'Content-Type':'application/json'}});

test('cloud load uses same-origin OAuth Bearer and validates authenticated tenant response', async () => {
  const requests=[];
  const client=clientWith(()=>json({project,head}),requests);
  assert.deepEqual(await client.load('tenant-a','hello'), {project,head});
  assert.equal(requests[0].url,'https://builder.example.test/v1/projects/hello');
  assert.equal(requests[0].options.headers.Authorization,'Bearer opaque-access-token');
  assert.equal(requests[0].options.credentials,'omit');
  assert.equal(requests[0].options.redirect,'error');
  assert.equal(requests[0].options.cache,'no-store');
});

test('cloud rejects cross-tenant response, invalid revision and invalid head', async () => {
  const client=clientWith(()=>json({project,head}),[]);
  await assert.rejects(client.load('tenant-b','hello'),/tenant/i);
  const broken=clientWith(()=>json({project,head:{revision:2,sha256:'x'}}),[]);
  await assert.rejects(broken.load('tenant-a','hello'),/head|revision/i);
});

test('cloud save sends CAS head, never cookies, and refuses revision gaps', async () => {
  const requests=[];
  const next={...project,revision:2};
  const client=clientWith(()=>json({revision:2,sha256:'b'.repeat(64)}),requests);
  const result=await client.save('tenant-a',next,head);
  assert.equal(result.revision,2);
  assert.deepEqual(JSON.parse(requests[0].options.body),{project:next,expected:head});
  assert.equal(requests[0].options.method,'PUT');
  await assert.rejects(client.save('tenant-a',{...next,revision:4},head),/revision/i);
  assert.equal(requests.length,1);
});

test('cloud reports 409 without retry or overwriting conflicting revisions', async () => {
  const requests=[];
  const client=clientWith(()=>json({error:'http_409'},409),requests);
  await assert.rejects(client.save('tenant-a',{...project,revision:2},head),e=>e instanceof CloudApiError && e.status===409);
  assert.equal(requests.length,1);
});

test('cloud preview performs authenticated inert POST and blocks invalid endpoints', async () => {
  const requests=[];
  const client=clientWith(()=>json({ordered_nodes:['input','output'],requested_tools:[],max_cost_minor:100,project_sha256:'b'.repeat(64)}),requests);
  const result=await client.preview('tenant-a',project);
  assert.deepEqual(result.ordered_nodes,['input','output']);
  assert.equal(requests[0].options.method,'POST');
  assert.equal(requests[0].url,'https://builder.example.test/v1/projects/preview');
  assert.throws(()=>new CloudWorkspaceClient({token:'x',origin:'http://builder.example.test'}),/https/i);
  assert.throws(()=>new CloudWorkspaceClient({token:'x',origin:'https://example.test/path'}),/origin/i);
  await assert.rejects(client.load('tenant-a','../other'),/identifier/i);
});
