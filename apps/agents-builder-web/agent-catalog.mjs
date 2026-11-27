/**
 * OCAF AB-OM-03/14: read-only source registry admission for Agents Builder.
 * No authority to execute, price, certify, license or install an agent.
 */
import {parseUniqueJson} from './strict-json.mjs';

const HEX40 = /^[0-9a-f]{40}$/;
const gitDigest = value => typeof value === 'string' && value.length === 40 && HEX40.test(value);
const ID = /^OCAF-AGENT-\d{3}$/;
const GROUP = /^G(?:[1-9]|[12]\d|3[0-3])$/;
const GROUP2 = /^G(?:0[1-9]|[12]\d|3[0-3])$/;
const AGENT_FIELDS = ['id','display_name','group_id','specialization','lifecycle',
  'deployment_status','artifact_status','documentation_status',
  'certification_status','price_status','executable'];
const SWARM_FIELDS = ['id','specialization','scope','lifecycle','deployment_status',
  'documentation_status','certification_status','price_status','executable'];
const ROOT_FIELDS = ['schema_version','provenance','fleet_id','active_runtime_agent_count',
  'source_agent_registry_git_blob','source_swarm_registry_git_blob','agents','swarms'];
const allowedText = (value, max=160) => typeof value === 'string' && value.length>0 &&
  value.length <= max && !/[\u0000-\u001f\u007f]/u.test(value);
const deny = reason => { throw new Error('catalog: '+reason); };
const exact = (value,fields) => value !== null && typeof value === 'object' &&
  !Array.isArray(value) && Object.keys(value).length === fields.length &&
  fields.every(key => Object.hasOwn(value,key));
const bounded = (value,max=160) => { if(!allowedText(value,max)) deny('invalid text'); };
const state = entry => {
  if (entry.deployment_status !== 'NOT_DEPLOYED' ||
      entry.documentation_status !== 'NOT_VERIFIED' ||
      entry.certification_status !== 'NOT_ISSUED' ||
      entry.price_status !== 'NOT_PRICED' || entry.executable !== false) {
    deny('unverified source cannot advertise deployment or commercial readiness');
  }
};

export function validatePublicCatalog(value) {
  if (!exact(value,ROOT_FIELDS) || value.schema_version !== 1 ||
      value.provenance !== 'GIT_SOURCE_INVENTORY_ONLY_NOT_EXECUTION_AUTHORITY' ||
      value.fleet_id !== 'OCAF-FLEET-111' || value.active_runtime_agent_count !== 0 ||
      !gitDigest(value.source_agent_registry_git_blob) ||
      !gitDigest(value.source_swarm_registry_git_blob) ||
      !Array.isArray(value.agents) || value.agents.length !== 111 ||
      !Array.isArray(value.swarms) || value.swarms.length !== 33) deny('source identity/counts');
  const ids = new Set(), names = new Set(), groups = new Set();
  for (const [index,swarm] of value.swarms.entries()) {
    if (!exact(swarm,SWARM_FIELDS) || !GROUP.test(swarm.id) ||
        swarm.id !== `G${index+1}` || groups.has(swarm.id)) deny('invalid swarm');
    bounded(swarm.specialization); bounded(swarm.scope,500);
    if (swarm.lifecycle !== 'REFERENCE_ONLY') deny('swarm lifecycle');
    state(swarm);groups.add(swarm.id);
  }
  for(const [index,agent] of value.agents.entries()) {
    if(!exact(agent,AGENT_FIELDS) || !ID.test(agent.id) ||
       agent.id !== `OCAF-AGENT-${String(index+1).padStart(3,'0')}` ||
       ids.has(agent.id) || names.has(agent.display_name) ||
       !GROUP2.test(agent.group_id) || !groups.has(`G${Number(agent.group_id.slice(1))}`))
      deny('agent identity, order or group');
    bounded(agent.display_name); bounded(agent.specialization);
    if (agent.lifecycle !== 'DRAFT' || agent.artifact_status !== 'NOT_BUILT')
      deny('agent lifecycle');
    state(agent);ids.add(agent.id);names.add(agent.display_name);
  }
  return value;
}

export function searchPublicCatalog(catalog, query, limit=30) {
  validatePublicCatalog(catalog);
  if (!Number.isSafeInteger(limit) || limit<1 || limit>50 ||
      typeof query !== 'string' || query.length>256) deny('invalid query or result limit');
  const needle=query.trim().toLocaleLowerCase('pl-PL');
  const all=[...catalog.agents,...catalog.swarms];
  return all.filter(x=>[x.id,x.display_name||'',x.specialization,
    x.group_id||'',x.scope||''].some(y=>y.toLocaleLowerCase('pl-PL').includes(needle))).slice(0,limit);
}

export async function fetchPublicCatalog({fetchImpl=globalThis.fetch.bind(globalThis),signal}={}) {
  const response=await fetchImpl('./agent-catalog.json',{
    method:'GET',credentials:'omit',redirect:'error',mode:'same-origin',
    cache:'no-store',referrerPolicy:'no-referrer',signal
  });
  const contentType=response.headers.get('content-type')||'';
  if (!response.ok || response.redirected || contentType.includes(',') ||
      contentType.split(';',1)[0].trim().toLowerCase() !== 'application/json')
    deny('untrusted source response');
  const declared=response.headers.get('content-length');
  if (declared!==null && (!/^\d+$/.test(declared) || Number(declared)>256*1024))
    deny('invalid response length');
  if (!response.body?.getReader) deny('response streaming unavailable');
  const chunks=[]; let total=0;
  const reader=response.body.getReader();
  try {
    for(;;) {
      const {value,done}=await reader.read();
      if(done)break;
      total+=value.byteLength;
      if(total>256*1024) deny('response body size limit');
      chunks.push(value);
    }
  } catch(error) { await reader.cancel().catch(()=>{});throw error; }
  const bytes=new Uint8Array(total);let offset=0;
  for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.byteLength;}
  let text;
  try { text=new TextDecoder('utf-8',{fatal:true}).decode(bytes); }
  catch { deny('invalid UTF-8'); }
  return validatePublicCatalog(parseUniqueJson(text,{maxBytes:256*1024}));
}
