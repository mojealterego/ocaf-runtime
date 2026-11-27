import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {generateCatalog, serializeCatalog} from '../build_public_agent_catalog.mjs';

const fixture=()=>{
  const agents=Array.from({length:111},(_,i)=>({
    agent_id:`OCAF-AGENT-${String(i+1).padStart(3,'0')}`,
    group_id:`G${String(Math.min(i+1,33)).padStart(2,'0')}`,
    specialization:'test-specialization',display_name:`Agent ${i+1}`,
    lifecycle:'DRAFT',deployment_status:'NOT_DEPLOYED',
    artifact_status:'NOT_BUILT',active_admission:'DENY',ostc_enabled:false,
    source_access:'disabled',model_backend:'UNASSIGNED'
  }));
  const groups=Array.from({length:33},(_,i)=>({id:`G${i+1}`,specialization:'test',scope:`Task ${i+1}`}));
  return {agents:{schema_version:1,fleet_id:'OCAF-FLEET-111',declared_agent_count:111,
    operational_swarm_count:33,active_runtime_agent_count:0,agents},swarms:{groups}};
};
const generate=data=>generateCatalog(Buffer.from(JSON.stringify(data.agents)),Buffer.from(JSON.stringify(data.swarms)));

test('source-backed 111 agents and 33 groups expose no fictional active or commercial state',()=>{
  const cat=generate(fixture());
  assert.equal(cat.agents.length,111);assert.equal(cat.swarms.length,33);
  assert.equal(cat.active_runtime_agent_count,0);
  assert.deepEqual(new Set(cat.agents.map(a=>a.id)).size,111);
  for(const entry of [...cat.agents,...cat.swarms]){
    assert.equal(entry.certification_status,'NOT_ISSUED');
    assert.equal(entry.price_status,'NOT_PRICED');
    assert.equal(entry.documentation_status,'NOT_VERIFIED');
    assert.equal(entry.executable,false);
    assert.equal(entry.deployment_status,'NOT_DEPLOYED');
  }
  assert.equal(cat.agents[0].display_name,'Agent 1');
  assert.equal(cat.swarms.at(-1).id,'G33');
  assert.deepEqual(JSON.parse(serializeCatalog(cat)),cat);
});

test('generation deterministic and digests pin exact input bytes',()=>{
  const data=fixture();const a=Buffer.from(JSON.stringify(data.agents));
  const b=Buffer.from(JSON.stringify(data.swarms));
  const result=generateCatalog(a,b);
  assert.equal(result.source_agent_registry_git_blob,
    createHash('sha1').update(`blob ${a.length}\0`).update(a).digest('hex'));
  assert.equal(serializeCatalog(result),serializeCatalog(generateCatalog(a,b)));
  data.agents.agents[0].display_name='Different';
  assert.notEqual(generate(data).source_agent_registry_git_blob,result.source_agent_registry_git_blob);
});

test('fails closed on any falsely active, built, certified-looking inventory',()=>{
  for(const [k,v] of [['lifecycle','ACTIVE'],['deployment_status','DEPLOYED'],
    ['artifact_status','BUILT'],['active_admission','ALLOW'],['ostc_enabled',true],
    ['source_access','enabled'],['model_backend','qwen']]){
    const f=fixture();f.agents.agents[0][k]=v;
    assert.throws(()=>generate(f),/catalog:.*agent/i,`${k} must deny`);
  }
  const f=fixture();f.agents.active_runtime_agent_count=1;
  assert.throws(()=>generate(f),/identities or counts/);
});

test('denies duplicate names, invalid group references and dropped records',()=>{
  const duplicate=fixture();duplicate.agents.agents[0].display_name='Agent 2';
  assert.throws(()=>generate(duplicate),/invalid identity/);
  const invalid=fixture();invalid.agents.agents[0].group_id='G99';
  assert.throws(()=>generate(invalid),/invalid identity/);
  const short=fixture();short.swarms.groups.pop();
  assert.throws(()=>generate(short),/identities or counts/);
  const swapped=fixture();[swapped.agents.agents[0],swapped.agents.agents[1]]=
    [swapped.agents.agents[1],swapped.agents.agents[0]];
  assert.throws(()=>generate(swapped),/invalid identity/);
});

test('rejects dangerous UTF8, large inventory and control character names',()=>{
  const invalid=fixture();invalid.agents.agents[0].display_name='\u0001attack';
  assert.throws(()=>generate(invalid),/invalid identity/);
  assert.throws(()=>generateCatalog(Buffer.from([0xff,0xfe]),Buffer.from('{}')),/invalid JSON or UTF-8/);
  assert.throws(()=>generateCatalog(Buffer.alloc(600000),Buffer.from('{}')),/size/);
});

test('denies duplicate JSON keys, escaped aliases, non-finite numbers and nested pollution',()=>{
  const fixtureData=fixture();
  const a=Buffer.from(JSON.stringify(fixtureData.agents));
  const b=Buffer.from(JSON.stringify(fixtureData.swarms));
  const malicious=[
    '{"schema_version":1,"schema_version":2}',
    '{"schema_version":1,"schema_\u0076ersion":1}',
    '{"nested":{"__proto__":1,"__proto__":2}}',
    '{"price":1e999}',
  ];
  for(const raw of malicious){
    assert.throws(()=>generateCatalog(Buffer.from(raw),b),
      /invalid JSON or UTF-8/, raw);
    assert.throws(()=>generateCatalog(a,Buffer.from(raw)),
      /invalid JSON or UTF-8/, raw);
  }
});
