import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const source = new URL('../entry/src/main/ets/services/BuilderDomain.ets', import.meta.url);
const dir = mkdtempSync(join(tmpdir(), 'ocaf-nocode-'));
writeFileSync(join(dir, 'BuilderDomain.mts'), readFileSync(source));
const core = await import(pathToFileURL(join(dir, 'BuilderDomain.mts')).href);
process.on('exit', () => rmSync(dir, { recursive: true, force: true }));
const tenant = 'tenant_a';
const fresh = () => core.newProject(tenant, 'agent');
const clone = p => JSON.parse(JSON.stringify(p));
const reverseKeys = value => Array.isArray(value) ? value.map(reverseKeys) :
  value !== null && typeof value === 'object' ? Object.fromEntries(Object.entries(value).reverse().map(([k,v]) => [k,reverseKeys(v)])) : value;

test('canonical bytes are invariant to every object key ordering', () => {
  const p = core.addTransform(fresh(),tenant,'step');
  assert.equal(core.writeCode(reverseKeys(p),tenant),core.writeCode(p,tenant));
  assert.equal(core.writeCode(core.readCode(JSON.stringify(reverseKeys(p)),tenant),tenant),core.writeCode(p,tenant));
});
test('depth limit rejects deeply nested input before decoding', () => {
  assert.throws(()=>core.readCode('['.repeat(65)+'0'+']'.repeat(65),tenant),/nesting depth/);
});
test('tool insertion accounts for grant, budget and revision without mutating source', () => {
  const p=fresh(), before=JSON.stringify(p), q=core.addTool(p,tenant,'tool1','search.tool',5);
  assert.equal(JSON.stringify(p),before);
  assert.deepEqual(q.grants,['search.tool']);
  assert.equal(q.max_cost_minor,5);
  assert.equal(q.revision,2);
  assert.deepEqual(core.compileProject(q,tenant).ordered_nodes,['input','tool1','output']);
  assert.equal(q.nodes.find(n=>n.id==='tool1').max_cost_minor,5);
});
test('shared grant retained until last requesting tool is removed', () => {
  let p=core.addTool(fresh(),tenant,'a','shared.tool',3);
  p=core.addTool(p,tenant,'b','shared.tool',7);
  assert.deepEqual(p.grants,['shared.tool']);
  p=core.removeStep(p,tenant,'a');
  assert.deepEqual(p.grants,['shared.tool']);assert.equal(p.max_cost_minor,7);
  p=core.removeStep(p,tenant,'b');
  assert.deepEqual(p.grants,[]);assert.equal(p.max_cost_minor,0);
  assert.deepEqual(core.compileProject(p,tenant).ordered_nodes,['input','output']);
  assert.equal(p.revision,5);
});
test('tool update adjusts cost and reconciles duplicate grants', () => {
  let p=core.addTool(fresh(),tenant,'a','old.tool',3);
  p=core.addTool(p,tenant,'b','old.tool',7);
  p=core.updateTool(p,tenant,'a','new.tool',10);
  assert.deepEqual(p.grants,['new.tool','old.tool']);assert.equal(p.max_cost_minor,17);
  p=core.updateTool(p,tenant,'b','new.tool',2);
  assert.deepEqual(p.grants,['new.tool']);assert.equal(p.max_cost_minor,12);
  assert.equal(p.revision,5);
});
test('rename updates all incident edges and refuses occupied IDs', () => {
  const p=core.addTransform(fresh(),tenant,'step');
  const q=core.renameNode(p,tenant,'step','renamed');
  assert.deepEqual(core.compileProject(q,tenant).ordered_nodes,['input','renamed','output']);
  assert.equal(q.edges.some(e=>e.from_node==='step'||e.to_node==='step'),false);
  assert.throws(()=>core.renameNode(q,tenant,'renamed','output'),/Invalid rename/);
  assert.equal(p.nodes.some(n=>n.id==='step'),true);
});
test('remove linear step maintains typed edge and unused budget', () => {
  let p=fresh();p.nodes[0].outputs[0].kind='bytes';p.nodes[1].inputs[0].kind='bytes';p.max_cost_minor=100;
  p=core.addTool(p,tenant,'tool','example',5);
  p=core.addTransform(p,tenant,'step');p=core.removeStep(p,tenant,'tool');
  assert.equal(p.max_cost_minor,100);p=core.removeStep(p,tenant,'step');
  assert.equal(p.nodes[0].outputs[0].kind,'bytes');assert.equal(p.nodes[1].inputs[0].kind,'bytes');
  assert.deepEqual(core.compileProject(p,tenant).ordered_nodes,['input','output']);
});
test('ambiguous branch removal rejects without changing the graph', () => {
  let p=core.addTransform(fresh(),tenant,'step');
  p.nodes[2].inputs.push({name:'other',kind:'text'});
  p.edges.push({from_node:'step',from_port:'out',to_node:'output',to_port:'other'});
  const before=JSON.stringify(p);
  core.compileProject(p,tenant);
  assert.throws(()=>core.removeStep(p,tenant,'step'),/lossiness/);assert.equal(JSON.stringify(p),before);
});
test('invalid costs, identity, revision and budget overflow fail closed', () => {
  const p=fresh(),before=JSON.stringify(p);
  for(const cost of [0,-1,0.5,NaN,Infinity,1000000000001])assert.throws(()=>core.addTool(p,tenant,'t','example',cost));
  assert.throws(()=>core.addTool(p,'foreign','t','example',1),/Tenant denied/);
  assert.throws(()=>core.removeStep(p,tenant,'input'),/lossiness/);
  assert.throws(()=>core.renameNode(p,tenant,'absent','new'),/Invalid rename/);
  assert.equal(JSON.stringify(p),before);
  const max=clone(p);max.max_cost_minor=1000000000000;
  assert.throws(()=>core.addTool(max,tenant,'t','example',1),/budget/);
  const old=clone(p);old.revision=Number.MAX_SAFE_INTEGER;
  assert.throws(()=>core.addTool(old,tenant,'t','example',1),/revision/);
});
test('repeatable Code/NoCode edit sequences keep typed canonical graph', () => {
  for(let i=0;i<32;i++){
    let p=fresh();p=core.addTool(p,tenant,'a','search.tool',i+1);p=core.addTransform(p,tenant,'b');
    p=core.renameNode(p,tenant,'b','step');p=core.updateTool(p,tenant,'a','other.tool',i+2);
    const serialized=core.writeCode(p,tenant);
    assert.equal(core.writeCode(core.readCode(serialized,tenant),tenant),serialized);
    assert.deepEqual(core.compileProject(p,tenant).ordered_nodes,['input','a','step','output']);
  }
});
