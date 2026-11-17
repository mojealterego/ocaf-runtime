import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname ?? '.', '..');
const sourcePath = resolve(root, 'entry/src/main/ets/services/BuilderDomain.ets');
const directory = mkdtempSync(join(tmpdir(), 'ocaf-harmony-domain-'));
writeFileSync(join(directory, 'BuilderDomain.mts'), readFileSync(sourcePath));
const core = await import(pathToFileURL(join(directory, 'BuilderDomain.mts')).href);
process.on('exit', () => rmSync(directory, { recursive: true, force: true }));
const clone = value => JSON.parse(JSON.stringify(value));
const fresh = () => core.newProject('tenant_a', 'agent_1');

test('Code NoCode roundtrip and sorted plan', () => {
  const before = fresh();
  assert.deepEqual(core.compileProject(before, 'tenant_a').ordered_nodes, ['input', 'output']);
  const next = core.addTransform(before, 'tenant_a', 'transform_1');
  assert.equal(before.revision, 1);
  assert.equal(next.revision, 2);
  assert.deepEqual(core.compileProject(next, 'tenant_a').ordered_nodes, ['input', 'transform_1', 'output']);
  assert.deepEqual(core.readCode(core.writeCode(next, 'tenant_a'), 'tenant_a'), next);
});

test('foreign tenant, spoofed IDs and unsupported schema denied', () => {
  assert.throws(() => core.compileProject(fresh(), 'tenant_b'), /Tenant denied/);
  assert.throws(() => core.newProject('tenant_a', '..'), /Invalid project ID/);
  for (const id of ['', 'bad/name', 'x'.repeat(65)]) {
    assert.throws(() => core.newProject('tenant_a', id), /Invalid project ID/);
  }
  const p = fresh(); p.schema_version = 2;
  assert.throws(() => core.compileProject(p, 'tenant_a'), /Invalid schema/);
});

test('untrusted extra fields and malformed or oversized code refused', () => {
  const p = clone(fresh()); p.user_is_admin = true;
  assert.throws(() => core.compileProject(p, 'tenant_a'), /Tenant denied/);
  assert.throws(() => core.readCode('{broken', 'tenant_a'), /Invalid JSON/);
  assert.throws(() => core.readCode(' '.repeat(131073), 'tenant_a'), /too large/);
});

test('cost, unknown tool grant and missing grant refused', () => {
  const p = fresh(); p.grants.push('unneeded');
  assert.throws(() => core.compileProject(p, 'tenant_a'), /Permission or budget/);
  const tool = core.addTransform(fresh(), 'tenant_a', 'tool_1');
  const node = tool.nodes.find(n => n.id === 'tool_1');
  node.kind = 'tool'; node.tool_name = 'search'; node.max_cost_minor = 5;
  tool.max_cost_minor = 4;
  tool.grants = ['search'];
  assert.throws(() => core.compileProject(tool, 'tenant_a'), /Permission or budget/);
  tool.max_cost_minor = 5;
  assert.deepEqual(core.compileProject(tool, 'tenant_a').requested_tools, ['search']);
  tool.grants = [];
  assert.throws(() => core.compileProject(tool, 'tenant_a'), /Permission or budget/);
});

test('typed edges, duplicate edges, cycle and missing ports denied', () => {
  const typeMismatch = fresh(); typeMismatch.nodes[1].inputs[0].kind = 'bytes';
  assert.throws(() => core.compileProject(typeMismatch, 'tenant_a'), /Port type mismatch/);
  const duplicate = fresh(); duplicate.edges.push(clone(duplicate.edges[0]));
  assert.throws(() => core.compileProject(duplicate, 'tenant_a'), /Duplicate/);
  const missing = fresh(); missing.nodes[1].inputs.push({name:'second',kind:'text'});
  assert.throws(() => core.compileProject(missing, 'tenant_a'), /Unbound input/);
  const cyclic = core.addTransform(core.addTransform(fresh(), 'tenant_a', 'a'), 'tenant_a', 'b');
  cyclic.nodes.find(n => n.id === 'a').inputs.push({name:'back',kind:'text'});
  cyclic.edges.push({from_node:'b',from_port:'out',to_node:'a',to_port:'back'});
  assert.throws(() => core.compileProject(cyclic, 'tenant_a'), /Cyclic|Invalid|disconnected/);
});

test('NoCode fails closed on conflicts and preserves input on failure', () => {
  const p = core.addTransform(fresh(), 'tenant_a', 'node');
  const snapshot = JSON.stringify(p);
  assert.throws(() => core.addTransform(p, 'tenant_a', 'node'), /Invalid transform/);
  assert.equal(JSON.stringify(p), snapshot);
  assert.throws(() => core.addTransform(p, 'tenant_a', 'bad/name'), /Invalid transform/);
  assert.equal(JSON.stringify(p), snapshot);
});

test('stable canonicalization across node/edge insertion order', () => {
  const p = core.addTransform(core.addTransform(fresh(), 'tenant_a','a'), 'tenant_a','b');
  const shuffled = clone(p); shuffled.nodes.reverse(); shuffled.edges.reverse();
  assert.equal(core.writeCode(shuffled, 'tenant_a'), core.writeCode(p, 'tenant_a'));
  for (let i=0; i<50; i++) {
    let q = fresh();
    for (let j=0; j<(i%7); j++) q = core.addTransform(q,'tenant_a','n_'+j);
    assert.deepEqual(core.readCode(core.writeCode(q,'tenant_a'),'tenant_a'), q);
  }
});
