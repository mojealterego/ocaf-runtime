import test from 'node:test';
import assert from 'node:assert/strict';
import {validatePublicCatalog, fetchPublicCatalog} from '../agent-catalog.mjs';

const fixture = () => ({
  schema_version:1, provenance:'GIT_SOURCE_INVENTORY_ONLY_NOT_EXECUTION_AUTHORITY',
  fleet_id:'OCAF-FLEET-111', active_runtime_agent_count:0,
  source_agent_registry_git_blob:'a'.repeat(40), source_swarm_registry_git_blob:'b'.repeat(40),
  agents:Array.from({length:111},(_,i)=>({
    id:`OCAF-AGENT-${String(i+1).padStart(3,'0')}`, display_name:`Agent ${i+1}`,
    group_id:'G01', specialization:'runtime', lifecycle:'DRAFT', artifact_status:'NOT_BUILT',
    deployment_status:'NOT_DEPLOYED', documentation_status:'NOT_VERIFIED',
    certification_status:'NOT_ISSUED', price_status:'NOT_PRICED', executable:false
  })),
  swarms:Array.from({length:33},(_,i)=>({
    id:`G${i+1}`, specialization:'runtime', scope:'reference', lifecycle:'REFERENCE_ONLY',
    deployment_status:'NOT_DEPLOYED', documentation_status:'NOT_VERIFIED',
    certification_status:'NOT_ISSUED', price_status:'NOT_PRICED', executable:false
  }))
});
const fetchWithType = type => fetchPublicCatalog({fetchImpl:async()=>new Response(
  JSON.stringify(fixture()),{headers:{'content-type':type}}
)});

test('AB-OM-03/14: rejects non-string Git provenance digests',()=>{
  for(const field of ['source_agent_registry_git_blob','source_swarm_registry_git_blob']){
    for(const value of [['a'.repeat(40)],null,123,{},'a'.repeat(40)+'\n']){
      const input=fixture(); input[field]=value;
      assert.throws(()=>validatePublicCatalog(input),/catalog:/,field);
    }
  }
  assert.equal(validatePublicCatalog(fixture()).agents.length,111);
});

test('AB-OM-03/14: rejects content types with a JSON prefix but different media type',async()=>{
  for(const type of ['application/jsonp','application/json-malicious','application/json, text/html']){
    await assert.rejects(fetchWithType(type),/catalog: untrusted source response/,type);
  }
});

test('AB-OM-03/14: accepts JSON media type with optional parameters and casing',async()=>{
  for(const type of ['application/json','Application/JSON; charset=utf-8']){
    assert.equal((await fetchWithType(type)).agents.length,111);
  }
});
