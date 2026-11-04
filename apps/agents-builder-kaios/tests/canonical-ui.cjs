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
  'canonical-remove': { onclick: null },
  'canonical-node-id': { value: 'step' },
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
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).ordered_nodes, ['input','step','output']);
elements['canonical-transform'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(JSON.parse(elements['canonical-source'].value).nodes.length, 3);
elements['canonical-remove'].onclick();
const afterRemoval = JSON.parse(elements['canonical-source'].value);
assert.deepEqual(afterRemoval.nodes.map(n => n.id), ['input','output']);
assert.deepEqual(JSON.parse(elements['canonical-result'].textContent).ordered_nodes, ['input','output']);
elements['canonical-remove'].onclick();
assert.match(elements['canonical-result'].textContent, /Błąd/);
assert.equal(JSON.parse(elements['canonical-source'].value).nodes.length, 2);
console.log('PASS canonical KaiOS UI validation, persistence, NoCode insert/remove');
