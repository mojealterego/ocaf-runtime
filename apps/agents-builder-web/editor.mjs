/**
 * OCAF 0.51 — browser Code/NoCode graph editor model (offline only).
 * This client-side validator mirrors the restricted Rust IR, but is NOT an
 * authorization boundary. Cloud admission MUST call Rust compile() using a
 * server-authenticated tenant. No tool execution or publishing is exposed.
 */
const SCHEMA = 1;
const MAX_BYTES = 128 * 1024;
const MAX_NODES = 64;
const MAX_EDGES = 256;
const MAX_PORTS = 8;
const MAX_GRANTS = 32;
const MAX_COST = 1_000_000_000_000;
const MAX_HISTORY = 32;
const KINDS = new Set(['input', 'output', 'transform', 'tool']);
const TYPES = new Set(['text', 'json', 'bytes']);
const KEYS = {
  project: ['schema_version','tenant_id','project_id','revision','max_cost_minor','grants','nodes','edges'],
  node: ['id','kind','inputs','outputs','tool_name','max_cost_minor'],
  port: ['name','kind'],
  edge: ['from_node','from_port','to_node','to_port']
};
const fault = (kind, detail = '') => { throw new Error(kind + (detail ? ': ' + detail : '')); };
const validId = value => typeof value === 'string' && value.length > 0 && value.length <= 64 && /^[A-Za-z0-9_.-]+$/.test(value);
const uint = value => Number.isSafeInteger(value) && value >= 0;
const clone = value => structuredClone(value);
const sorted = items => [...items].sort();
const strict = (obj, keys) => {
  if (!obj || typeof obj !== 'object' || Array.isArray(obj)) fault('schema: expected object');
  if (Object.keys(obj).length !== keys.length || keys.some(key => !Object.hasOwn(obj, key))) fault('schema: unsupported fields');
};
const id = value => { if (!validId(value)) fault('identifier: invalid'); };
const requireArray = value => { if (!Array.isArray(value)) fault('schema: expected array'); };
const boundedNumber = value => { if (!uint(value)) fault('schema: invalid integer'); };
const byteLength = text => new TextEncoder().encode(text).length;

function walk(start, adjacency) {
  const seen = new Set(), stack = [start];
  while (stack.length) {
    const next = stack.pop();
    if (seen.has(next)) continue;
    seen.add(next);
    stack.push(...(adjacency.get(next) || []));
  }
  return seen;
}

export function validateProject(project, trustedTenant) {
  strict(project, KEYS.project);
  id(trustedTenant);
  if (project.tenant_id !== trustedTenant) fault('tenant: project ownership denied');
  id(project.project_id);
  if (project.schema_version !== SCHEMA || !uint(project.revision) || project.revision === 0) fault('schema: version or revision');
  boundedNumber(project.max_cost_minor);
  requireArray(project.nodes); requireArray(project.edges); requireArray(project.grants);
  if (project.nodes.length < 2 || project.nodes.length > MAX_NODES || project.edges.length > MAX_EDGES || project.grants.length > MAX_GRANTS || project.max_cost_minor > MAX_COST) fault('limit: graph or budget');
  const grants = new Set();
  for (const grant of project.grants) {
    id(grant);
    if (grants.has(grant)) fault('permission: duplicate grant');
    grants.add(grant);
  }
  const nodes = new Map(), requested = new Set(), inputs = [], outputs = [];
  let cost = 0;
  for (const node of project.nodes) {
    strict(node, KEYS.node); id(node.id);
    if (!KINDS.has(node.kind) || nodes.has(node.id)) fault('schema: node kind or duplicate');
    nodes.set(node.id, node);
    boundedNumber(node.max_cost_minor);
    requireArray(node.inputs); requireArray(node.outputs);
    for (const ports of [node.inputs, node.outputs]) {
      if (ports.length > MAX_PORTS) fault('limit: ports');
      const names = new Set();
      for (const port of ports) {
        strict(port, KEYS.port); id(port.name);
        if (!TYPES.has(port.kind) || names.has(port.name)) fault('schema: port');
        names.add(port.name);
      }
    }
    if (node.kind === 'input') {
      if (node.inputs.length || !node.outputs.length || node.tool_name !== null || node.max_cost_minor !== 0) fault('schema: input node');
      inputs.push(node.id);
    } else if (node.kind === 'output') {
      if (!node.inputs.length || node.outputs.length || node.tool_name !== null || node.max_cost_minor !== 0) fault('schema: output node');
      outputs.push(node.id);
    } else if (node.kind === 'transform') {
      if (!node.inputs.length || !node.outputs.length || node.tool_name !== null || node.max_cost_minor !== 0) fault('schema: transform node');
    } else {
      if (!node.inputs.length || !node.outputs.length || !validId(node.tool_name) || node.max_cost_minor === 0) fault('schema: tool node');
      requested.add(node.tool_name);
      cost += node.max_cost_minor;
      if (!Number.isSafeInteger(cost) || cost > MAX_COST) fault('limit: cost');
    }
  }
  if (inputs.length !== 1 || outputs.length !== 1) fault('schema: exactly one input and output required');
  if (cost > project.max_cost_minor) fault('budget: tool costs exceed project maximum');
  if (requested.size !== grants.size || [...requested].some(name => !grants.has(name))) fault('permission: missing or unused tool grant');

  const degrees = new Map([...nodes.keys()].map(key => [key, 0]));
  const children = new Map(), parents = new Map(), incoming = new Set(), edges = new Set();
  for (const edge of project.edges) {
    strict(edge, KEYS.edge);
    const signature = [edge.from_node,edge.from_port,edge.to_node,edge.to_port].join('\u0000');
    const target = nodes.get(edge.to_node), source = nodes.get(edge.from_node);
    if (!target || !source || edge.from_node === edge.to_node || edges.has(signature)) fault('graph: invalid or duplicate edge');
    edges.add(signature);
    const from = source.outputs.find(port => port.name === edge.from_port);
    const to = target.inputs.find(port => port.name === edge.to_port);
    if (!from || !to) fault('graph: port missing');
    if (from.kind !== to.kind) fault('type: mismatched edge');
    const sink = edge.to_node + '\u0000' + edge.to_port;
    if (incoming.has(sink)) fault('graph: fan-in to one port');
    incoming.add(sink);
    degrees.set(edge.to_node, degrees.get(edge.to_node) + 1);
    if (!children.has(edge.from_node)) children.set(edge.from_node, []);
    if (!parents.has(edge.to_node)) parents.set(edge.to_node, []);
    children.get(edge.from_node).push(edge.to_node);
    parents.get(edge.to_node).push(edge.from_node);
  }
  for (const node of project.nodes) {
    for (const port of node.inputs) {
      if (!incoming.has(node.id + '\u0000' + port.name)) fault('graph: unconnected required input');
    }
  }
  const ready = sorted([...degrees].filter(([,degree]) => degree === 0).map(([name]) => name));
  const order = [];
  while (ready.length) {
    const name = ready.shift();
    order.push(name);
    for (const child of children.get(name) || []) {
      const degree = degrees.get(child) - 1;
      degrees.set(child, degree);
      if (degree === 0) { ready.push(child); ready.sort(); }
    }
  }
  if (order.length !== nodes.size) fault('graph: cycle');
  if (walk(inputs[0], children).size !== nodes.size || walk(outputs[0], parents).size !== nodes.size) fault('graph: unreachable node');
  return {order, tools: sorted(requested), cost};
}

export function canonicalProject(project, trustedTenant) {
  const {order} = validateProject(project, trustedTenant);
  const result = clone(project), ranks = new Map(order.map((name, index) => [name,index]));
  result.nodes.sort((a,b) => ranks.get(a.id) - ranks.get(b.id));
  for (const node of result.nodes) {
    node.inputs.sort((a,b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
    node.outputs.sort((a,b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
  }
  result.edges.sort((a,b) => {
    for (const key of KEYS.edge) {
      if (a[key] < b[key]) return -1;
      if (a[key] > b[key]) return 1;
    }
    return 0;
  });
  result.grants.sort();
  return result;
}

export function encodeDsl(project, trustedTenant) {
  const result = JSON.stringify(canonicalProject(project, trustedTenant), null, 2);
  if (byteLength(result) > MAX_BYTES) fault('limit: DSL exceeds 128 KiB');
  return result;
}

export function decodeDsl(source, trustedTenant) {
  if (typeof source !== 'string') fault('schema: DSL must be text');
  if (byteLength(source) > MAX_BYTES) fault('limit: DSL exceeds 128 KiB');
  let parsed;
  try { parsed = JSON.parse(source); } catch { fault('json: invalid or unsupported Code DSL'); }
  const result = canonicalProject(parsed, trustedTenant);
  if (byteLength(JSON.stringify(result)) > MAX_BYTES) fault('limit: project exceeds 128 KiB');
  return result;
}

export function newProject(tenant, projectId) {
  id(tenant); id(projectId);
  return {
    schema_version: SCHEMA, tenant_id: tenant, project_id: projectId,
    revision: 1, max_cost_minor: 100, grants: [],
    nodes: [
      {id:'input',kind:'input',inputs:[],outputs:[{name:'out',kind:'text'}],tool_name:null,max_cost_minor:0},
      {id:'output',kind:'output',inputs:[{name:'in',kind:'text'}],outputs:[],tool_name:null,max_cost_minor:0}
    ],
    edges: [{from_node:'input',from_port:'out',to_node:'output',to_port:'in'}]
  };
}

const grantsFor = nodes => sorted(new Set(nodes.filter(n => n.kind === 'tool').map(n => n.tool_name)));

export function insertNode(project, kind, nodeId, options = {}) {
  validateProject(project, project.tenant_id);
  id(nodeId);
  if (kind !== 'transform' && kind !== 'tool') fault('node: only transform and tool can be inserted');
  if (project.nodes.some(node => node.id === nodeId)) fault('node: duplicate ID');
  const next = clone(project);
  const edgeIndex = options.edgeIndex ?? 0;
  if (!uint(edgeIndex) || edgeIndex >= next.edges.length) fault('graph: invalid insertion edge');
  const edge = next.edges[edgeIndex];
  const source = next.nodes.find(n => n.id === edge.from_node);
  const target = next.nodes.find(n => n.id === edge.to_node);
  const sourcePort = source.outputs.find(p => p.name === edge.from_port);
  const targetPort = target.inputs.find(p => p.name === edge.to_port);
  if (sourcePort.kind !== targetPort.kind) fault('type: invalid insertion');
  const tool = kind === 'tool';
  const toolName = tool ? options.tool_name : null;
  const cost = tool ? options.max_cost_minor : 0;
  if (tool && (!validId(toolName) || !uint(cost) || cost === 0)) fault('node: invalid tool settings');
  next.nodes.push({
    id: nodeId, kind,
    inputs: [{name:'in',kind:sourcePort.kind}],
    outputs: [{name:'out',kind:sourcePort.kind}],
    tool_name: toolName, max_cost_minor: cost
  });
  next.edges.splice(edgeIndex, 1,
    {from_node:edge.from_node,from_port:edge.from_port,to_node:nodeId,to_port:'in'},
    {from_node:nodeId,from_port:'out',to_node:edge.to_node,to_port:edge.to_port}
  );
  next.grants = grantsFor(next.nodes);
  next.revision += 1;
  validateProject(next, next.tenant_id);
  return next;
}

export function removeNode(project, nodeId) {
  validateProject(project, project.tenant_id);
  const node = project.nodes.find(n => n.id === nodeId);
  if (!node || !['transform','tool'].includes(node.kind)) fault('node: cannot remove endpoint');
  const inbound = project.edges.filter(e => e.to_node === nodeId);
  const outbound = project.edges.filter(e => e.from_node === nodeId);
  if (inbound.length !== 1 || outbound.length !== 1 || node.inputs.length !== 1 || node.outputs.length !== 1) fault('graph: only single-port linear nodes can be removed');
  const next = clone(project);
  next.nodes = next.nodes.filter(n => n.id !== nodeId);
  next.edges = next.edges.filter(e => e.from_node !== nodeId && e.to_node !== nodeId);
  next.edges.push({
    from_node:inbound[0].from_node,from_port:inbound[0].from_port,
    to_node:outbound[0].to_node,to_port:outbound[0].to_port
  });
  next.grants = grantsFor(next.nodes);
  next.revision += 1;
  validateProject(next, next.tenant_id);
  return next;
}

export function setTool(project, nodeId, toolName, maxCostMinor) {
  validateProject(project, project.tenant_id);
  id(toolName);
  if (!uint(maxCostMinor) || maxCostMinor === 0) fault('node: invalid tool cost');
  const next = clone(project);
  const node = next.nodes.find(n => n.id === nodeId);
  if (!node || node.kind !== 'tool') fault('node: not a tool');
  node.tool_name = toolName;
  node.max_cost_minor = maxCostMinor;
  next.grants = grantsFor(next.nodes);
  next.revision += 1;
  validateProject(next, next.tenant_id);
  return next;
}

// Offline browser storage is a convenience feature, NOT authenticated tenancy,
// durable cloud storage or a transactional cross-tab database.
const storageKey = (tenant, projectId) => {
  id(tenant); id(projectId);
  return 'ocaf.builder.offline.v1:' + tenant + ':' + projectId;
};
const readRecord = (storage, tenant, projectId) => {
  const raw = storage.getItem(storageKey(tenant, projectId));
  if (raw === null) return null;
  if (typeof raw !== 'string' || byteLength(raw) > MAX_BYTES * MAX_HISTORY) fault('limit: offline history');
  let record;
  try { record = JSON.parse(raw); } catch { fault('schema: corrupt offline history'); }
  if (!record || !Array.isArray(record.history) || record.history.length === 0 || record.history.length > MAX_HISTORY) fault('schema: offline history');
  let last = 0;
  for (const snapshot of record.history) {
    validateProject(snapshot, tenant);
    if (snapshot.project_id !== projectId || snapshot.revision <= last) fault('schema: invalid offline revision chain');
    last = snapshot.revision;
  }
  return record;
};

export function saveProject(storage, project, expectedRevision) {
  validateProject(project, project.tenant_id);
  boundedNumber(expectedRevision);
  const record = readRecord(storage, project.tenant_id, project.project_id);
  const current = record?.history.at(-1)?.revision ?? 0;
  if (current !== expectedRevision || project.revision !== current + 1) fault('conflict: stale revision');
  const history = [...(record?.history ?? []), canonicalProject(project, project.tenant_id)].slice(-MAX_HISTORY);
  const payload = JSON.stringify({schema_version:SCHEMA, history});
  if (byteLength(payload) > MAX_BYTES * MAX_HISTORY) fault('limit: offline history');
  storage.setItem(storageKey(project.tenant_id, project.project_id), payload);
  return project.revision;
}

export function loadProject(storage, tenant, projectId) {
  const record = readRecord(storage, tenant, projectId);
  return record ? clone(record.history.at(-1)) : null;
}

export function listRevisions(storage, tenant, projectId) {
  return (readRecord(storage, tenant, projectId)?.history ?? []).map(snapshot => snapshot.revision);
}

export function restoreRevision(storage, tenant, projectId, revision) {
  boundedNumber(revision);
  const record = readRecord(storage, tenant, projectId);
  if (!record) fault('node: project missing');
  const original = record.history.find(snapshot => snapshot.revision === revision);
  if (!original) fault('node: revision not found');
  const latest = record.history.at(-1);
  const restored = clone(original);
  restored.revision = latest.revision + 1;
  saveProject(storage, restored, latest.revision);
  return restored;
}
