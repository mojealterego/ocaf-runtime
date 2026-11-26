#!/usr/bin/env node
/**
 * Deterministically publishes a public, NON-AUTHORITATIVE roster from two
 * Git-tracked OCAF inventory files. Never fabricates certifications, prices,
 * ACTIVE state or authorization to execute/deploy.
 */
import {readFile, writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve, join} from 'node:path';
import {fileURLToPath} from 'node:url';

export const AGENT_REGISTRY = 'docs/agents/agent_registry_111.json';
export const SWARM_REGISTRY = 'docs/agents/swarm_groups_v1.json';
export const CATALOG_PATH = 'apps/agents-builder-web/agent-catalog.json';
const IDENTIFIER = /^[A-Za-z0-9_.-]{1,96}$/;
const text = (value, max=160) => typeof value === 'string' &&
    value.length > 0 && value.length <= max && !/[\u0000-\u001f\u007f]/u.test(value);
const invalid = reason => { throw new Error('catalog: ' + reason); };
const gitBlob = bytes => createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
const jsonFile = (bytes, name) => {
  if (bytes.length < 2 || bytes.length > 512 * 1024) invalid(name + ' size');
  try { return JSON.parse(new TextDecoder('utf-8', {fatal:true}).decode(bytes)); }
  catch { invalid(name + ' invalid JSON or UTF-8'); }
};

export function generateCatalog(agentBytes, swarmBytes) {
  const agentSource = jsonFile(agentBytes, 'agent registry');
  const swarmSource = jsonFile(swarmBytes, 'swarm registry');
  if (agentSource.schema_version !== 1 || agentSource.fleet_id !== 'OCAF-FLEET-111' ||
      agentSource.declared_agent_count !== 111 || agentSource.operational_swarm_count !== 33 ||
      agentSource.active_runtime_agent_count !== 0 ||
      !Array.isArray(agentSource.agents) || agentSource.agents.length !== 111 ||
      !Array.isArray(swarmSource.groups) || swarmSource.groups.length !== 33) {
    invalid('source identities or counts');
  }
  const groups = new Set();
  const swarms = swarmSource.groups.map((group, index) => {
    if (!group || group.id !== `G${index+1}` || !text(group.specialization) ||
        !text(group.scope, 500) || groups.has(group.id)) invalid('invalid swarm');
    groups.add(group.id);
    return {
      id:group.id, specialization:group.specialization, scope:group.scope,
      lifecycle:'REFERENCE_ONLY', deployment_status:'NOT_DEPLOYED',
      documentation_status:'NOT_VERIFIED', certification_status:'NOT_ISSUED',
      price_status:'NOT_PRICED', executable:false
    };
  });
  const agentIds = new Set(), names = new Set();
  const agents = agentSource.agents.map((entry, index) => {
    const id = `OCAF-AGENT-${String(index+1).padStart(3, '0')}`;
    if (!entry || entry.agent_id !== id || agentIds.has(entry.agent_id) ||
        !text(entry.display_name) || names.has(entry.display_name) ||
        !text(entry.specialization) || !IDENTIFIER.test(entry.specialization) ||
        !/^G(?:0[1-9]|[12][0-9]|3[0-3])$/.test(entry.group_id) ||
        !groups.has(`G${Number(entry.group_id.slice(1))}`) ||
        entry.lifecycle !== 'DRAFT' || entry.deployment_status !== 'NOT_DEPLOYED' ||
        entry.artifact_status !== 'NOT_BUILT' || entry.active_admission !== 'DENY' ||
        entry.ostc_enabled !== false || entry.source_access !== 'disabled' ||
        entry.model_backend !== 'UNASSIGNED') invalid('unverified agent or invalid identity: '+id);
    names.add(entry.display_name); agentIds.add(entry.agent_id);
    return {
      id, display_name:entry.display_name, group_id:entry.group_id,
      specialization:entry.specialization, lifecycle:'DRAFT',
      deployment_status:'NOT_DEPLOYED', artifact_status:'NOT_BUILT',
      documentation_status:'NOT_VERIFIED', certification_status:'NOT_ISSUED',
      price_status:'NOT_PRICED', executable:false
    };
  });
  return {
    schema_version:1, provenance:'GIT_SOURCE_INVENTORY_ONLY_NOT_EXECUTION_AUTHORITY',
    fleet_id:'OCAF-FLEET-111', active_runtime_agent_count:0,
    source_agent_registry_git_blob:gitBlob(agentBytes),
    source_swarm_registry_git_blob:gitBlob(swarmBytes),
    agents, swarms
  };
}

export const serializeCatalog = value => JSON.stringify(value, null, 2) + '\n';

async function run() {
  const options = process.argv.slice(2);
  let repo = resolve(fileURLToPath(new URL('../', import.meta.url)));
  let check = false;
  for (let i=0;i<options.length;i++) {
    if (options[i] === '--check') check = true;
    else if (options[i] === '--repo' && options[i+1]) repo=resolve(options[++i]);
    else invalid('unknown CLI argument');
  }
  const [rawAgents,rawSwarms] = await Promise.all([
    readFile(join(repo, AGENT_REGISTRY)), readFile(join(repo, SWARM_REGISTRY))
  ]);
  const next = serializeCatalog(generateCatalog(rawAgents,rawSwarms));
  const target = join(repo,CATALOG_PATH);
  if (check) {
    const existing=await readFile(target,'utf8');
    if (existing !== next) invalid('checked-in catalogue is stale or modified');
  } else {
    await writeFile(target,next,{flag:'w'});
  }
  console.log(`Catalog ${check?'verified':'generated'}: 111 DRAFT agents; 33 reference swarms; ACTIVE=0`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  run().catch(err => {console.error(err.message);process.exitCode=1;});
}
