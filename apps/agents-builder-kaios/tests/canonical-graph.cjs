'use strict';
const assert = require('node:assert/strict');
const { compile } = require('../canonical-graph.js').OCAFCanonicalGraph;
function project() {
  return {
    schema_version: 1, tenant_id: 'tenant', project_id: 'p', revision: 1,
    max_cost_minor: 0, grants: [],
    nodes: [
      { id: 'in', kind: 'input', inputs: [], outputs: [{name:'out',kind:'text'}], tool_name:null,max_cost_minor:0 },
      { id: 'out', kind: 'output', inputs: [{name:'in',kind:'text'}], outputs: [],tool_name:null,max_cost_minor:0 }
    ],
    edges: [{from_node:'in',from_port:'out',to_node:'out',to_port:'in'}]
  };
}
assert.deepEqual(compile(project(),'tenant').ordered_nodes,['in','out']);
assert.throws(() => compile(project(),'other'), /Tenant denied/);
let p=project(); p.edges[0].from_port='missing';
assert.throws(() => compile(p,'tenant'), /Invalid edge port/);
p=project(); p.nodes[1].inputs[0].kind='json';
assert.throws(() => compile(p,'tenant'), /Port type mismatch/);
p=project(); p.edges=[];
assert.throws(() => compile(p,'tenant'), /Unconnected input/);
p=project(); p.nodes[1].extra=true;
assert.throws(() => compile(p,'tenant'), /Invalid node/);
p=project(); p.grants=['unrequested'];
assert.throws(() => compile(p,'tenant'), /Permission mismatch/);
p=project();
p.nodes.splice(1,0,{id:'step',kind:'transform',inputs:[{name:'in',kind:'text'}],outputs:[{name:'out',kind:'text'}],tool_name:null,max_cost_minor:0});
p.edges=[{from_node:'in',from_port:'out',to_node:'step',to_port:'in'},
         {from_node:'step',from_port:'out',to_node:'out',to_port:'in'}];
assert.deepEqual(compile(p,'tenant').ordered_nodes,['in','step','out']);

// Additional negative contract cases for canonical graph safety.
p=project(); p.revision=0;
assert.throws(() => compile(p,'tenant'), /Invalid project/);
p=project(); p.max_cost_minor=1000000000001;
assert.throws(() => compile(p,'tenant'), /Invalid budget/);
p=project(); p.nodes[0].outputs[0].kind='bytes'; p.nodes[1].inputs[0].kind='bytes';
assert.deepEqual(compile(p,'tenant').ordered_nodes,['in','out']);
p=project(); p.nodes[0].outputs[0].kind='json'; p.nodes[1].inputs[0].kind='json';
assert.deepEqual(compile(p,'tenant').ordered_nodes,['in','out']);
p=project(); p.edges.push({...p.edges[0]});
assert.throws(() => compile(p,'tenant'), /Duplicate edge/);
p=project(); p.nodes.push({id:'orphan',kind:'transform',inputs:[{name:'in',kind:'text'}],outputs:[{name:'out',kind:'text'}],tool_name:null,max_cost_minor:0});
assert.throws(() => compile(p,'tenant'), /Unconnected input/);
p=project(); p.nodes[1].id='__proto__'; p.edges[0].to_node='__proto__';
assert.deepEqual(compile(p,'tenant').ordered_nodes,['__proto__','in']);

console.log('PASS canonical KaiOS graph validation: 15 assertions');
