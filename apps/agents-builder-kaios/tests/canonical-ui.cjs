'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const root = path.join(__dirname, '..');
const elements = {
  'canonical-check': { onclick: null },
  'canonical-save': { onclick: null },
  'canonical-load': { onclick: null },
  'canonical-new': { onclick: null },
  'canonical-transform': { onclick: null },
  'canonical-tool': { onclick: null },
  'canonical-rename': { onclick: null },
  'canonical-duplicate': { onclick: null },
  'canonical-new-node-id': { value: 'renamed' },
  'canonical-remove-tool': { onclick: null },
  'canonical-tool-name': { value: 'search.tool' },
  'canonical-tool-cost': { value: '7' },
  'canonical-remove': { onclick: null },
  'canonical-export': { onclick: null },
  'canonical-import-button': { onclick: null },
  'canonical-import': { value: '' },
  'canonical-node-id': { value: 'step' },
  'canonical-port-kind': { value: 'text' },
  'canonical-source': { value: '' },
  'canonical-tenant': { value: 'local' },
  'canonical-id': { value: 'agent' },
  'canonical-result': { textContent: '' }
};
const storage = new Map();
const context = { document: { getElementById(id) { return elements[id]; } }, localStorage: {
  getItem(key) { return storage.get(key) || null; },
  setItem(key,value) { storage.set(key,value); }
} };
vm.createContext(context);
vm.runInContext(fs.readFileSync(path.join(root, 'canonical-graph.js'), 'utf8'), context);
vm.runInContext(fs.readFileSync(path.join(root, 'canonical-ui.js'), 'utf8'), context);
const project = {
  schema_version:1, tenant_id:'local', project_id:'agent', revision:1,
  max_cost_minor:0, grants:[],
  nodes:[
    {id:'start',kind:'input',inputs:[],outputs:[{name:'out',kind:'text'}],tool_name:null,max_cost_minor:0},
    {id:'finish',kind:'output',inputs:[{name:'in',kind:'text'}],outputs:[],tool_name:null,max_cost_minor:0}
  ],
  edges:[{from_node:'start',from_port:'out',to_node:'finish',to_port:'in'}]
};
elements['canonical-source'].value = JSON.stringify(project);
elements['canonical-check'].onclick();
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).ordered_nodes, ['start','finish']);
elements['canonical-tenant'].value = 'other';
elements['canonical-check'].onclick();
assert.match(elements['canonical-result'].textContent, /Tenant denied/);
elements['canonical-tenant'].value = 'local';
project.edges[0].to_port = 'wrong';
elements['canonical-source'].value = JSON.stringify(project);
elements['canonical-check'].onclick();
assert.match(elements['canonical-result'].textContent, /Invalid edge port/);
project.edges[0].to_port = 'in';
elements['canonical-source'].value = JSON.stringify(project);
elements['canonical-save'].onclick();
assert.match(elements['canonical-result'].textContent, /Zapisano/);
elements['canonical-source'].value = '';
elements['canonical-load'].onclick();
assert.equal(JSON.parse(elements['canonical-source'].value).project_id, 'agent');
project.edges[0].to_port = 'wrong';
elements['canonical-source'].value = JSON.stringify(project);
elements['canonical-save'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(JSON.parse(storage.get('ocaf-kaios-canonical-v1:local:agent')).edges[0].to_port, 'in');
elements['canonical-new'].onclick();
assert.deepEqual(JSON.parse(elements['canonical-source'].value).nodes.map(n => n.id), ['input','output']);
elements['canonical-node-id'].value = 'step';
elements['canonical-transform'].onclick();
const transformed = JSON.parse(elements['canonical-source'].value);
assert.deepEqual(transformed.nodes.map(n => n.id), ['input','output','step']);
assert.equal(transformed.edges.length, 2);
assert.equal(transformed.revision, 2);
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).ordered_nodes, ['input','step','output']);
elements['canonical-transform'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(JSON.parse(elements['canonical-source'].value).nodes.length, 3);
elements['canonical-remove'].onclick();
const afterRemoval = JSON.parse(elements['canonical-source'].value);
assert.deepEqual(afterRemoval.nodes.map(n => n.id), ['input','output']);
assert.equal(afterRemoval.revision, 3);
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).ordered_nodes, ['input','output']);
elements['canonical-remove'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(JSON.parse(elements['canonical-source'].value).nodes.length, 2);
elements['canonical-port-kind'].value = 'json';
elements['canonical-new'].onclick();
let typed = JSON.parse(elements['canonical-source'].value);
assert.equal(typed.nodes[0].outputs[0].kind, 'json');
assert.equal(typed.nodes[1].inputs[0].kind, 'json');
elements['canonical-transform'].onclick();
typed = JSON.parse(elements['canonical-source'].value);
assert.equal(typed.nodes[2].inputs[0].kind, 'json');
assert.equal(typed.nodes[2].outputs[0].kind, 'json');
assert.equal(typed.revision, 2);
elements['canonical-remove'].onclick();
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).ordered_nodes, ['input','output']);
elements['canonical-port-kind'].value = 'bytes';
elements['canonical-new'].onclick();
assert.equal(JSON.parse(elements['canonical-source'].value).nodes[0].outputs[0].kind, 'bytes');
elements['canonical-port-kind'].value = 'invalid';
const previousSource = elements['canonical-source'].value;
elements['canonical-new'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(elements['canonical-source'].value, previousSource);
elements['canonical-port-kind'].value = 'text';
elements['canonical-new'].onclick();
elements['canonical-export'].onclick();
const exported = elements['canonical-import'].value;
assert.equal(JSON.parse(exported).project_id, 'agent');
elements['canonical-source'].value = '';
elements['canonical-import-button'].onclick();
assert.equal(JSON.parse(elements['canonical-source'].value).project_id, 'agent');
elements['canonical-import'].value = exported.replace('"tenant_id": "local"', '"tenant_id": "other"');
const beforeRejectedImport = elements['canonical-source'].value;
elements['canonical-import-button'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(elements['canonical-source'].value, beforeRejectedImport);
elements['canonical-port-kind'].value = 'text';
elements['canonical-new'].onclick();
elements['canonical-node-id'].value = 'tool1';
elements['canonical-tool'].onclick();
let withTool = JSON.parse(elements['canonical-source'].value);
assert.equal(withTool.nodes.filter(n => n.kind === 'tool').length, 1);
assert.deepEqual(withTool.grants, ['search.tool']);
assert.equal(withTool.max_cost_minor, 7);
assert.equal(withTool.revision, 2);
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).requested_tools, ['search.tool']);
elements['canonical-node-id'].value = 'tool2';
elements['canonical-tool-cost'].value = '0';
const beforeBadTool = elements['canonical-source'].value;
elements['canonical-tool'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(elements['canonical-source'].value, beforeBadTool);
elements['canonical-node-id'].value = 'tool1';
elements['canonical-remove-tool'].onclick();
let withoutTool = JSON.parse(elements['canonical-source'].value);
assert.equal(withoutTool.nodes.filter(n => n.kind === 'tool').length, 0);
assert.deepEqual(withoutTool.grants, []);
assert.equal(withoutTool.max_cost_minor, 0);
assert.equal(withoutTool.revision, 3);
elements['canonical-remove-tool'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
// Reusing a tool grant must not create duplicate grants.
elements['canonical-new'].onclick();
elements['canonical-node-id'].value = 'toolA';
elements['canonical-tool-name'].value = 'shared.tool';
elements['canonical-tool-cost'].value = '3';
elements['canonical-tool'].onclick();
elements['canonical-node-id'].value = 'toolB';
elements['canonical-tool'].onclick();
assert.deepEqual(JSON.parse(elements['canonical-source'].value).grants, ['shared.tool']);
assert.equal(JSON.parse(elements['canonical-source'].value).max_cost_minor, 6);
assert.equal(JSON.parse(elements['canonical-source'].value).revision, 3);
elements['canonical-node-id'].value = 'toolB';
elements['canonical-remove-tool'].onclick();
assert.deepEqual(JSON.parse(elements['canonical-source'].value).grants, ['shared.tool']);
assert.equal(JSON.parse(elements['canonical-source'].value).max_cost_minor, 3);
elements['canonical-node-id'].value = 'toolA';
elements['canonical-remove-tool'].onclick();
assert.deepEqual(JSON.parse(elements['canonical-source'].value).grants, []);
assert.equal(JSON.parse(elements['canonical-source'].value).max_cost_minor, 0);
const validSaved = storage.get('ocaf-kaios-canonical-v1:local:agent');
storage.set('ocaf-kaios-canonical-v1:local:agent', '{');
const beforeCorruptLoad = elements['canonical-source'].value;
elements['canonical-load'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(elements['canonical-source'].value, beforeCorruptLoad);
storage.set('ocaf-kaios-canonical-v1:local:agent', validSaved);
elements['canonical-import'].value = '{';
const beforeMalformedImport = elements['canonical-source'].value;
elements['canonical-import-button'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(elements['canonical-source'].value, beforeMalformedImport);
elements['canonical-new'].onclick();
elements['canonical-node-id'].value = 'input';
elements['canonical-new-node-id'].value = 'source';
elements['canonical-rename'].onclick();
let renamed = JSON.parse(elements['canonical-source'].value);
assert.deepEqual(renamed.nodes.map(n => n.id), ['source', 'output']);
assert.equal(renamed.edges[0].from_node, 'source');
assert.equal(renamed.revision, 2);
elements['canonical-node-id'].value = 'source';
elements['canonical-new-node-id'].value = 'output';
const beforeBadRename = elements['canonical-source'].value;
elements['canonical-rename'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(elements['canonical-source'].value, beforeBadRename);
elements['canonical-new'].onclick();
elements['canonical-node-id'].value = 'first';
elements['canonical-transform'].onclick();
elements['canonical-node-id'].value = 'second';
elements['canonical-new-node-id'].value = 'first';
elements['canonical-duplicate'].onclick();
const duplicated = JSON.parse(elements['canonical-source'].value);
assert.deepEqual(duplicated.nodes.map(n => n.id), ['input','output','first','second']);
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).ordered_nodes, ['input','first','second','output']);
assert.equal(duplicated.revision, 3);
elements['canonical-duplicate'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(elements['canonical-source'].value, JSON.stringify(duplicated, null, 2));
console.log('PASS canonical KaiOS UI typed ports, persistence, import/export, rename, duplicate');
