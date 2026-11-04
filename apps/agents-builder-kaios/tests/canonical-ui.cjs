'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const root = path.join(__dirname, '..');
const elements = {
  'canonical-check': { onclick: null },
  'canonical-source': { value: '' },
  'canonical-tenant': { value: 'local' },
  'canonical-result': { textContent: '' }
};
const context = { document: { getElementById(id) { return elements[id]; } } };
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
console.log('PASS canonical KaiOS UI validation, tenant isolation, invalid edge');
