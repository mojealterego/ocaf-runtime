import test from 'node:test';
import assert from 'node:assert/strict';
import {
  newProject, insertNode, removeNode, setTool, validateProject,
  encodeDsl, decodeDsl, saveProject, loadProject, listRevisions, restoreRevision
} from '../editor.mjs';

const storage = () => {
  const data = new Map();
  return {
    getItem: key => data.get(key) ?? null,
    setItem: (key, value) => { data.set(key, String(value)); },
    removeItem: key => data.delete(key)
  };
};

test('valid new project has a complete typed Input -> Output graph', () => {
  const project = newProject('local', 'hello');
  assert.equal(project.revision, 1);
  assert.deepEqual(validateProject(project, 'local').order, ['input', 'output']);
});

test('Code JSON -> NoCode graph -> Code JSON roundtrip is canonical', () => {
  const project = insertNode(newProject('local', 'hello'), 'transform', 'normalize');
  const encoded = encodeDsl(project, 'local');
  const decoded = decodeDsl(encoded, 'local');
  assert.deepEqual(decoded, JSON.parse(encoded));
  const reordered = structuredClone(project);
  reordered.nodes.reverse();
  reordered.edges.reverse();
  assert.equal(encodeDsl(reordered, 'local'), encoded);
});

test('tool nodes require explicit grants and bounded cost', () => {
  const project = insertNode(newProject('local', 'hello'), 'tool', 'search', {
    tool_name: 'tool_search', max_cost_minor: 12
  });
  assert.deepEqual(project.grants, ['tool_search']);
  assert.deepEqual(validateProject(project, 'local').tools, ['tool_search']);
  const withoutGrant = structuredClone(project);
  withoutGrant.grants = [];
  assert.throws(() => validateProject(withoutGrant, 'local'), /permission/i);
  const overBudget = structuredClone(project);
  overBudget.max_cost_minor = 11;
  assert.throws(() => validateProject(overBudget, 'local'), /budget/i);
});

test('type mismatch, cycle, fan-in and unknown fields fail closed', () => {
  const project = insertNode(newProject('local', 'hello'), 'transform', 'step');
  const mismatch = structuredClone(project);
  mismatch.nodes[1].inputs[0].kind = 'json';
  assert.throws(() => validateProject(mismatch, 'local'), /type/i);
  const cyclic = structuredClone(project);
  cyclic.edges.push({from_node:'step',from_port:'out',to_node:'step',to_port:'in'});
  assert.throws(() => validateProject(cyclic, 'local'), /graph/i);
  const duplicate = structuredClone(project);
  duplicate.edges.push(structuredClone(duplicate.edges[0]));
  assert.throws(() => validateProject(duplicate, 'local'), /graph/i);
  assert.throws(() => decodeDsl(encodeDsl(project, 'local').replace('"revision": 2', '"revision": 2, "evil": true'), 'local'), /schema/i);
});

test('untrusted tenant and arbitrary source are rejected, not silently converted', () => {
  const project = newProject('local', 'hello');
  assert.throws(() => validateProject(project, 'another'), /tenant/i);
  assert.throws(() => decodeDsl('fn main() {}', 'local'), /json|schema/i);
  assert.throws(() => decodeDsl('x'.repeat(131073), 'local'), /limit/i);
});

test('graph edits preserve typed connectivity and remove obsolete grants', () => {
  const project = insertNode(newProject('local', 'hello'), 'tool', 'search', {
    tool_name: 'tool_search', max_cost_minor: 5
  });
  const updated = setTool(project, 'search', 'tool_index', 8);
  assert.deepEqual(updated.grants, ['tool_index']);
  const removed = removeNode(updated, 'search');
  assert.deepEqual(removed.grants, []);
  assert.deepEqual(validateProject(removed, 'local').order, ['input', 'output']);
  assert.throws(() => removeNode(removed, 'input'), /node/i);
});

test('offline revision store supports optimistic conflict detection and durable revert', () => {
  const db = storage();
  const original = newProject('local', 'hello');
  saveProject(db, original, 0);
  const next = insertNode(original, 'transform', 'step');
  saveProject(db, next, 1);
  assert.equal(loadProject(db, 'local', 'hello').revision, 2);
  assert.deepEqual(listRevisions(db, 'local', 'hello'), [1, 2]);
  assert.throws(() => saveProject(db, next, 1), /conflict/i);
  const restored = restoreRevision(db, 'local', 'hello', 1);
  assert.equal(restored.revision, 3);
  assert.deepEqual(validateProject(restored, 'local').order, ['input', 'output']);
  assert.deepEqual(listRevisions(db, 'local', 'hello'), [1, 2, 3]);
  assert.equal(loadProject(db, 'local', 'missing'), null);
});

test('browser-local namespace separates project records but is not authentication', () => {
  const db = storage();
  saveProject(db, newProject('local', 'hello'), 0);
  assert.equal(loadProject(db, 'other', 'hello'), null);
  assert.throws(() => newProject('../evil', 'hello'), /identifier/i);
});

test('Rust fixture is accepted by the web Code/NoCode converter', async () => {
  const fs = await import('node:fs/promises');
  const fixture = await fs.readFile(new URL('../fixtures/hello.json', import.meta.url), 'utf8');
  assert.deepEqual(validateProject(decodeDsl(fixture, 'local'), 'local').order, ['input', 'output']);
});

test('Code DSL canonical port ordering matches Rust ASCII identifier ordering', () => {
  const project = newProject('local', 'sort');
  project.nodes[0].outputs.push(
    {name:'Z',kind:'text'}, {name:'a',kind:'text'}, {name:'_',kind:'text'}
  );
  const encoded = JSON.parse(encodeDsl(project, 'local'));
  assert.deepEqual(encoded.nodes[0].outputs.map(port => port.name), ['Z','_','a','out']);
});
