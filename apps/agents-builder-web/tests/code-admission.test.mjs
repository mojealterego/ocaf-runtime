import assert from 'node:assert/strict';
import { test } from 'node:test';
import * as editor from '../editor.mjs';
const fresh=()=>editor.newProject('local','demo');
const encode=p=>editor.encodeDsl(p,'local');
const reverseKeys=v=>Array.isArray(v)?v.map(reverseKeys):v!==null&&typeof v==='object'?Object.fromEntries(Object.entries(v).reverse().map(([k,x])=>[k,reverseKeys(x)])):v;
const apply=(source,revision=1,tenant='local',id='demo')=>editor.applyCodeRevision(source,tenant,id,revision);
test('canonical bytes do not depend on key order at any DAG level',()=>{
  const p=editor.insertNode(fresh(),'tool','tool1',{tool_name:'example.tool',max_cost_minor:5});
  assert.equal(encode(reverseKeys(p)),encode(p));
  assert.equal(encode(editor.decodeDsl(JSON.stringify(reverseKeys(p)),'local')),encode(p));
});
test('Apply Code rejects duplicate tenant, revision and nested keys before normalization',()=>{
  const s=encode(fresh());
  for(const invalid of [s.replace('"tenant_id": "local"','"tenant_id": "foreign", "tenant_id": "local"'),
    s.replace('"tenant_id": "local"','"tenant_id": "foreign", "ten\\u0061nt_id": "local"'),
    s.replace('"revision": 1','"revision": 9, "revision": 1'),
    s.replace('"kind": "input"','"kind": "tool", "kind": "input"')]){
    assert.throws(()=>apply(invalid),/json/i);
  }
});
test('Apply Code validates user document even if the revision is server-controlled',()=>{
  const p=fresh();p.revision='bad';assert.throws(()=>apply(JSON.stringify(p)),/revision/);
  p.revision=42;const next=apply(JSON.stringify(p),7);assert.equal(next.revision,8);
  assert.equal(p.revision,42);
});
test('identity and revision overflow fail closed',()=>{
  const p=fresh(),source=encode(p);
  assert.throws(()=>apply(source,1,'foreign'),/tenant/);
  assert.throws(()=>apply(source,1,'local','other'),/identity/);
  for(const rev of [0,-1,Number.MAX_SAFE_INTEGER,1.5,NaN])assert.throws(()=>apply(source,rev),/revision/);
});
test('valid Code draft continues to persist with the existing offline CAS contract',()=>{
  const data=new Map(),storage={getItem:k=>data.get(k)??null,setItem:(k,v)=>data.set(k,v)};
  const p=fresh();editor.saveProject(storage,p,0);p.max_cost_minor=222;
  const next=apply(JSON.stringify(reverseKeys(p)));editor.saveProject(storage,next,1);
  assert.equal(editor.loadProject(storage,'local','demo').max_cost_minor,222);
  assert.deepEqual(editor.listRevisions(storage,'local','demo'),[1,2]);
});
