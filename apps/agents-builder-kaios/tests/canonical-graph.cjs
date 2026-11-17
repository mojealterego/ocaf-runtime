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
assert.deepEqual(compile(p,'tenant').ordered_nodes,['in','__proto__']);

console.log('PASS canonical KaiOS graph validation: 15 assertions');

const { parseUniqueJson } = require('../canonical-graph.js').OCAFCanonicalGraph;
assert.deepEqual(parseUniqueJson('{"outer":{"name":"x"},"other":{"name":"y"}}',16384),{outer:{name:"x"},other:{name:"y"}});
assert.throws(() => parseUniqueJson('{"tenant_id":"x","ten\\u0061nt_id":"y"}',131072), /duplicate/);
assert.throws(() => parseUniqueJson('{"nodes":[{"id":"x","id":"y"}]}',131072), /duplicate/);
assert.throws(() => parseUniqueJson('{"__proto__":1,"__proto__":2}',16384), /duplicate/);
assert.deepEqual(parseUniqueJson('{"name":"quoted \\"x\\"","other":"{[}"}',16384),{name:'quoted "x"',other:'{[}'});
assert.throws(() => parseUniqueJson('"' + 'ż'.repeat(8192) + '"',16384), /16 KiB/);
assert.throws(() => parseUniqueJson('"' + '😀'.repeat(4096) + '"',16384), /16 KiB/);
assert.throws(() => parseUniqueJson('['.repeat(65)+'0'+']'.repeat(65),131072), /depth/);
assert.throws(() => parseUniqueJson('{"x":1,}',16384), /JSON|property|position|token/i);
assert.equal(parseUniqueJson('"'+'ż'.repeat(8191)+'"',16384).length,8191);
assert.equal(parseUniqueJson('"'+'a'.repeat(131070)+'"',131072).length,131070);
console.log('PASS KaiOS unique JSON / UTF-8 / depth admission: 11 assertions');
