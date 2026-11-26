import test from 'node:test';
import assert from 'node:assert/strict';
import {validatePublicCatalog, searchPublicCatalog, fetchPublicCatalog} from '../agent-catalog.mjs';

const fixture=()=>({
  schema_version:1,
  provenance:'GIT_SOURCE_INVENTORY_ONLY_NOT_EXECUTION_AUTHORITY',
  fleet_id:'OCAF-FLEET-111',active_runtime_agent_count:0,
  source_agent_registry_git_blob:'a'.repeat(40),
  source_swarm_registry_git_blob:'b'.repeat(40),
  agents:Array.from({length:111},(_,i)=>({
    id:`OCAF-AGENT-${String(i+1).padStart(3,'0')}`,display_name:`Test Agent ${i+1}`,
    group_id:`G${String(Math.min(i+1,33)).padStart(2,'0')}`,
    specialization:'orchestration-runtime',lifecycle:'DRAFT',
    deployment_status:'NOT_DEPLOYED',artifact_status:'NOT_BUILT',
    documentation_status:'NOT_VERIFIED',certification_status:'NOT_ISSUED',
    price_status:'NOT_PRICED',executable:false
  })),
  swarms:Array.from({length:33},(_,i)=>({
    id:`G${i+1}`,specialization:'orchestration-runtime',scope:`Group task ${i+1}`,
    lifecycle:'REFERENCE_ONLY',deployment_status:'NOT_DEPLOYED',
    documentation_status:'NOT_VERIFIED',certification_status:'NOT_ISSUED',
    price_status:'NOT_PRICED',executable:false
  }))
});

test('accepts source-only 111 agent + 33 swarm catalog; never implies runnable or priced',()=>{
  const catalog=validatePublicCatalog(fixture());
  assert.equal(catalog.agents.length,111);
  assert.equal(catalog.swarms.length,33);
  assert.ok(catalog.agents.every(x=>x.executable === false && x.certification_status === 'NOT_ISSUED'));
  assert.deepEqual(searchPublicCatalog(catalog,'test agent 1',5).map(x=>x.id).slice(0,2),['OCAF-AGENT-001','OCAF-AGENT-010']);
  assert.deepEqual(searchPublicCatalog(catalog,'orchestration-runtime',2).length,2);
});

test('rejects forged active, certificate, price and artifact statuses',()=>{
  for (const [field, value] of [
    ['lifecycle','ACTIVE'],['deployment_status','DEPLOYED'],['executable',true],
    ['certification_status','ISSUED'],['price_status','VERIFIED'],
    ['documentation_status','VERIFIED'],['artifact_status','BUILT']
  ]){
    const bad=fixture();bad.agents[0][field]=value;
    assert.throws(()=>validatePublicCatalog(bad),/catalog:/,field);
  }
  const bad=fixture();bad.active_runtime_agent_count=1;
  assert.throws(()=>validatePublicCatalog(bad),/catalog:/);
});

test('rejects missing/duplicate/swapped records, cross-group forgery, unknown fields',()=>{
  for(const field of ['agents','swarms']){
    const missing=fixture(); missing[field].pop();
    assert.throws(()=>validatePublicCatalog(missing),/catalog:/);
  }
  const duplicates=fixture(); duplicates.agents[1].display_name=duplicates.agents[0].display_name;
  assert.throws(()=>validatePublicCatalog(duplicates),/catalog:/);
  const order=fixture(); [order.agents[0],order.agents[1]]=[order.agents[1],order.agents[0]];
  assert.throws(()=>validatePublicCatalog(order),/catalog:/);
  const groups=fixture();groups.agents[0].group_id='G99';
  assert.throws(()=>validatePublicCatalog(groups),/catalog:/);
  const unknown=fixture();unknown.agents[0].api_key='secret';
  assert.throws(()=>validatePublicCatalog(unknown),/catalog:/);
});

test('rejects HTML control characters and invalid provenance',()=>{
  const bad=fixture();bad.agents[0].display_name='A\nB';
  assert.throws(()=>validatePublicCatalog(bad),/catalog:/);
  const forged=fixture();forged.source_agent_registry_git_blob='not-a-digest';
  assert.throws(()=>validatePublicCatalog(forged),/catalog:/);
  assert.throws(()=>searchPublicCatalog(fixture(),'x',0),/catalog:/);
});

test('fetches only bounded same-origin inventory with no credentials',async()=>{
  const got=[];
  const fake=async(url,options)=>{
    got.push([url,options]);
    return new Response(JSON.stringify(fixture()),{status:200,headers:{'content-type':'application/json'}});
  };
  const result=await fetchPublicCatalog({fetchImpl:fake});
  assert.equal(result.agents.length,111);
  assert.equal(got[0][0],'./agent-catalog.json');
  assert.equal(got[0][1].credentials,'omit');
  assert.equal(got[0][1].redirect,'error');
});

test('rejects JSON duplicate keys, bad content type and overlarge body',async()=>{
  await assert.rejects(fetchPublicCatalog({fetchImpl:async()=>new Response('{"schema_version":1,"schema_version":1}',{headers:{'content-type':'application/json'}})}),/json|catalog:/i);
  await assert.rejects(fetchPublicCatalog({fetchImpl:async()=>new Response('{}',{headers:{'content-type':'text/html'}})}),/catalog:/);
  await assert.rejects(fetchPublicCatalog({fetchImpl:async()=>new Response('x'.repeat(270000),{headers:{'content-type':'application/json'}})}),/limit|catalog:/);
});
