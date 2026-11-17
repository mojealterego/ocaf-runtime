/**
 * OCAF Agents Builder: shared strictly bounded JSON DAG contract for HarmonyOS.
 * This is a local validation/authoring module, NOT a tool executor or auth gateway.
 */
export type PortType = 'text' | 'json' | 'bytes';
export type NodeKind = 'input' | 'output' | 'transform' | 'tool';
export interface Port { name: string; kind: PortType }
export interface GraphNode {
  id: string; kind: NodeKind; inputs: Port[]; outputs: Port[];
  tool_name: string | null; max_cost_minor: number;
}
export interface GraphEdge {
  from_node: string; from_port: string; to_node: string; to_port: string;
}
export interface BuilderProject {
  schema_version: number; tenant_id: string; project_id: string;
  revision: number; max_cost_minor: number; grants: string[];
  nodes: GraphNode[]; edges: GraphEdge[];
}
export interface CompiledGraph {
  ordered_nodes: string[]; requested_tools: string[]; max_cost_minor: number;
}
const MAX_BYTES = 128 * 1024;
const MAX_COST = 1_000_000_000_000;
/** Codepoint ordering matches Rust BTreeMap ordering for ASCII identifiers. */
function byteOrder(a: string, b: string): number { return a < b ? -1 : a > b ? 1 : 0; }
function utf8Length(value: string): number {
  let length = 0;
  for (let i = 0; i < value.length; i++) {
    const c = value.charCodeAt(i);
    if (c <= 0x7f) length++;
    else if (c <= 0x7ff) length += 2;
    else if (c >= 0xd800 && c <= 0xdbff && i + 1 < value.length &&
             value.charCodeAt(i + 1) >= 0xdc00 && value.charCodeAt(i + 1) <= 0xdfff) {
      length += 4; i++;
    } else length += 3;
    if (length > MAX_BYTES) return length;
  }
  return length;
}
/** Reject structurally ambiguous duplicate JSON keys, including escaped aliases. */
function rejectDuplicateJsonKeys(source: string): void {
  const frames: Array<{ object: boolean; keys: Set<string> }> = [];
  for (let i = 0; i < source.length; i++) {
    const c = source[i];
    if (c === '{' || c === '[') {
      if (frames.length >= 64) fail('JSON nesting depth limit');
      frames.push({ object: c === '{', keys: new Set<string>() });
    }
    else if (c === '}' || c === ']') frames.pop();
    else if (c === '"') {
      const start = i;
      let escaped = false;
      i++;
      for (; i < source.length; i++) {
        const next = source[i];
        if (escaped) { escaped = false; continue; }
        if (next === '\\') { escaped = true; continue; }
        if (next === '"') break;
      }
      if (i >= source.length) return; // JSON.parse reports malformed strings.
      let j = i + 1;
      while (j < source.length && /\s/.test(source[j])) j++;
      const current = frames[frames.length - 1];
      if (source[j] === ':' && current && current.object) {
        const name = JSON.parse(source.slice(start, i + 1)) as string;
        if (current.keys.has(name)) fail('Duplicate JSON property');
        current.keys.add(name);
      }
    }
  }
}

const TYPES: PortType[] = ['text', 'json', 'bytes'];
const KINDS: NodeKind[] = ['input', 'output', 'transform', 'tool'];
const PROJECT_KEYS = ['schema_version','tenant_id','project_id','revision','max_cost_minor','grants','nodes','edges'];
const NODE_KEYS = ['id','kind','inputs','outputs','tool_name','max_cost_minor'];
const EDGE_KEYS = ['from_node','from_port','to_node','to_port'];
const PORT_KEYS = ['name','kind'];
function fail(message: string): never { throw new Error(message); }
function validId(value: unknown): boolean {
  return typeof value === 'string' && /^[A-Za-z0-9_.-]{1,64}$/.test(value) && value !== '.' && value !== '..';
}
function uint(value: unknown): boolean { return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0; }
function objectKeys(value: unknown, names: string[]): boolean {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return false;
  const keys = Object.keys(value);
  return keys.length === names.length && names.every((key: string) => keys.includes(key));
}
function validatePorts(ports: Port[]): void {
  if (!Array.isArray(ports) || ports.length > 8) fail('Invalid ports');
  const seen = new Set<string>();
  ports.forEach((port: Port) => {
    if (!objectKeys(port, PORT_KEYS) || !validId(port.name) || !TYPES.includes(port.kind) || seen.has(port.name)) {
      fail('Invalid or duplicate port');
    }
    seen.add(port.name);
  });
}
function walk(start: string, links: Map<string, string[]>): Set<string> {
  const seen = new Set<string>();
  const stack: string[] = [start];
  while (stack.length > 0) {
    const item = stack.pop()!;
    if (seen.has(item)) continue;
    seen.add(item);
    for (const next of links.get(item) ?? []) stack.push(next);
  }
  return seen;
}
/** The tenant parameter must come from a trusted ingress for server enforcement. */
export function compileProject(project: BuilderProject, trustedTenant: string): CompiledGraph {
  if (!objectKeys(project, PROJECT_KEYS) || !validId(trustedTenant) || project.tenant_id !== trustedTenant) fail('Tenant denied');
  if (project.schema_version !== 1 || !validId(project.project_id) || !uint(project.revision) || project.revision < 1) fail('Invalid schema or revision');
  if (!uint(project.max_cost_minor) || project.max_cost_minor > MAX_COST ||
      !Array.isArray(project.nodes) || project.nodes.length < 2 || project.nodes.length > 64 ||
      !Array.isArray(project.edges) || project.edges.length > 256 ||
      !Array.isArray(project.grants) || project.grants.length > 32) fail('Resource bound');
  const grants = new Set<string>();
  for (const grant of project.grants) {
    if (!validId(grant) || grants.has(grant)) fail('Invalid or duplicate permission');
    grants.add(grant);
  }
  const nodes = new Map<string, GraphNode>();
  const children = new Map<string, string[]>();
  const parents = new Map<string, string[]>();
  const degrees = new Map<string, number>();
  const tools = new Set<string>();
  let totalCost = 0;
  let input = ''; let output = '';
  for (const node of project.nodes) {
    if (!objectKeys(node, NODE_KEYS) || !validId(node.id) || nodes.has(node.id) || !KINDS.includes(node.kind) || !uint(node.max_cost_minor)) fail('Invalid node');
    validatePorts(node.inputs); validatePorts(node.outputs);
    if (node.kind === 'input') {
      if (input || node.inputs.length !== 0 || node.outputs.length === 0 || node.tool_name !== null || node.max_cost_minor !== 0) fail('Invalid input');
      input = node.id;
    } else if (node.kind === 'output') {
      if (output || node.inputs.length === 0 || node.outputs.length !== 0 || node.tool_name !== null || node.max_cost_minor !== 0) fail('Invalid output');
      output = node.id;
    } else if (node.kind === 'transform') {
      if (node.inputs.length === 0 || node.outputs.length === 0 || node.tool_name !== null || node.max_cost_minor !== 0) fail('Invalid transform');
    } else {
      if (node.inputs.length === 0 || node.outputs.length === 0 || !validId(node.tool_name) || node.max_cost_minor === 0) fail('Invalid tool');
      tools.add(node.tool_name!);
      totalCost += node.max_cost_minor;
      if (!Number.isSafeInteger(totalCost) || totalCost > MAX_COST) fail('Cost overflow');
    }
    nodes.set(node.id, node); children.set(node.id, []); parents.set(node.id, []); degrees.set(node.id, 0);
  }
  if (!input || !output || totalCost > project.max_cost_minor ||
      grants.size !== tools.size || Array.from(tools).some((tool: string) => !grants.has(tool))) fail('Permission or budget denied');
  const edges = new Set<string>();
  const incoming = new Set<string>();
  for (const edge of project.edges) {
    if (!objectKeys(edge, EDGE_KEYS) || !validId(edge.from_node) || !validId(edge.to_node) ||
        !validId(edge.from_port) || !validId(edge.to_port) || edge.from_node === edge.to_node) fail('Invalid edge');
    const source = nodes.get(edge.from_node); const target = nodes.get(edge.to_node);
    if (!source || !target) fail('Unknown node');
    const from = source.outputs.find((port: Port) => port.name === edge.from_port);
    const to = target.inputs.find((port: Port) => port.name === edge.to_port);
    if (!from || !to) fail('Unknown port');
    if (from.kind !== to.kind) fail('Port type mismatch');
    const key = JSON.stringify(edge);
    const slot = edge.to_node + '\u0000' + edge.to_port;
    if (edges.has(key) || incoming.has(slot)) fail('Duplicate or ambiguous edge');
    edges.add(key); incoming.add(slot);
    children.get(edge.from_node)!.push(edge.to_node);
    parents.get(edge.to_node)!.push(edge.from_node);
    degrees.set(edge.to_node, degrees.get(edge.to_node)! + 1);
  }
  for (const node of project.nodes) for (const port of node.inputs) {
    if (!incoming.has(node.id + '\u0000' + port.name)) fail('Unbound input port');
  }
  const ready: string[] = Array.from(degrees.keys()).filter((key: string) => degrees.get(key) === 0).sort();
  const ordered: string[] = [];
  while (ready.length > 0) {
    const id = ready.shift()!; ordered.push(id);
    for (const next of children.get(id)!) {
      degrees.set(next, degrees.get(next)! - 1);
      if (degrees.get(next) === 0) { ready.push(next); ready.sort(); }
    }
  }
  if (ordered.length !== nodes.size || walk(input, children).size !== nodes.size ||
      walk(output, parents).size !== nodes.size) fail('Cyclic or disconnected graph');
  return { ordered_nodes: ordered, requested_tools: Array.from(tools).sort(), max_cost_minor: project.max_cost_minor };
}
export function canonicalProject(project: BuilderProject, tenant: string): BuilderProject {
  const compiled = compileProject(project, tenant);
  // Reconstruct every record in Rust struct declaration order, not input key order.
  const copy: BuilderProject = {
    schema_version: project.schema_version, tenant_id: project.tenant_id,
    project_id: project.project_id, revision: project.revision,
    max_cost_minor: project.max_cost_minor, grants: project.grants.slice(),
    nodes: project.nodes.map((node: GraphNode): GraphNode => ({
      id: node.id, kind: node.kind,
      inputs: node.inputs.map((port: Port): Port => ({ name: port.name, kind: port.kind })),
      outputs: node.outputs.map((port: Port): Port => ({ name: port.name, kind: port.kind })),
      tool_name: node.tool_name, max_cost_minor: node.max_cost_minor
    })),
    edges: project.edges.map((edge: GraphEdge): GraphEdge => ({
      from_node: edge.from_node, from_port: edge.from_port, to_node: edge.to_node, to_port: edge.to_port
    }))
  };
  const order = new Map<string, number>();
  compiled.ordered_nodes.forEach((id: string, index: number) => order.set(id, index));
  copy.nodes.sort((a: GraphNode, b: GraphNode) => order.get(a.id)! - order.get(b.id)!);
  for (const node of copy.nodes) {
    node.inputs.sort((a: Port, b: Port) => byteOrder(a.name, b.name));
    node.outputs.sort((a: Port, b: Port) => byteOrder(a.name, b.name));
  }
  copy.edges.sort((a: GraphEdge, b: GraphEdge) =>
    byteOrder(JSON.stringify([a.from_node, a.from_port, a.to_node, a.to_port]),
      JSON.stringify([b.from_node, b.from_port, b.to_node, b.to_port])));
  copy.grants.sort();
  return copy;
}
export function readCode(source: string, tenant: string): BuilderProject {
  if (typeof source !== 'string' || utf8Length(source) > MAX_BYTES) fail('DSL too large');
  let parsed: BuilderProject;
  rejectDuplicateJsonKeys(source);
  try { parsed = JSON.parse(source) as BuilderProject; }
  catch (_error) { return fail('Invalid JSON Code DSL'); }
  return canonicalProject(parsed, tenant);
}
export function writeCode(project: BuilderProject, tenant: string): string {
  const source = JSON.stringify(canonicalProject(project, tenant), null, 2);
  if (utf8Length(source) > MAX_BYTES) fail('DSL too large');
  return source;
}
export function newProject(tenant: string, projectId: string): BuilderProject {
  if (!validId(tenant) || !validId(projectId)) fail('Invalid project ID');
  return {
    schema_version: 1, tenant_id: tenant, project_id: projectId, revision: 1,
    max_cost_minor: 0, grants: [],
    nodes: [
      { id: 'input', kind: 'input', inputs: [], outputs: [{ name: 'out', kind: 'text' }], tool_name: null, max_cost_minor: 0 },
      { id: 'output', kind: 'output', inputs: [{ name: 'in', kind: 'text' }], outputs: [], tool_name: null, max_cost_minor: 0 }
    ],
    edges: [{ from_node: 'input', from_port: 'out', to_node: 'output', to_port: 'in' }]
  };
}
export function addTransform(project: BuilderProject, tenant: string, nodeId: string): BuilderProject {
  compileProject(project, tenant);
  if (!validId(nodeId) || project.nodes.some((n: GraphNode) => n.id === nodeId) ||
      project.revision >= Number.MAX_SAFE_INTEGER) fail('Invalid transform ID or revision');
  // NoCode's safe subset: exactly one edge entering output, one typed input.
  const output = project.nodes.find((n: GraphNode) => n.kind === 'output')!;
  const candidates = project.edges.filter((e: GraphEdge) => e.to_node === output.id);
  if (candidates.length !== 1 || output.inputs.length !== 1) fail('NoCode conversion lossiness: multi-input output');
  const edge = candidates[0];
  const type = output.inputs[0].kind;
  const copy = JSON.parse(JSON.stringify(project)) as BuilderProject;
  copy.nodes.push({ id: nodeId, kind: 'transform', inputs: [{ name: 'in', kind: type }],
    outputs: [{ name: 'out', kind: type }], tool_name: null, max_cost_minor: 0 });
  copy.edges = copy.edges.filter((e: GraphEdge) => !(e.from_node === edge.from_node &&
    e.from_port === edge.from_port && e.to_node === edge.to_node && e.to_port === edge.to_port));
  copy.edges.push({from_node: edge.from_node, from_port: edge.from_port, to_node: nodeId, to_port: 'in'});
  copy.edges.push({from_node: nodeId, from_port: 'out', to_node: output.id, to_port: edge.to_port});
  copy.revision++;
  return canonicalProject(copy, tenant);
}

/** Declarative graph authoring only. The grants below are not runtime authorization. */
function reconcileGrants(project: BuilderProject): void {
  const grants = new Set<string>();
  for (const node of project.nodes) if (node.kind === 'tool' && node.tool_name !== null) grants.add(node.tool_name);
  project.grants = Array.from(grants).sort();
}
function editCopy(project: BuilderProject, tenant: string): BuilderProject {
  const copy = canonicalProject(project, tenant);
  if (copy.revision >= Number.MAX_SAFE_INTEGER) fail('Invalid revision');
  copy.revision++;
  return copy;
}
function checkedCost(cost: number): void {
  if (!uint(cost) || cost < 1 || cost > MAX_COST) fail('Invalid tool cost');
}
export function addTool(project: BuilderProject, tenant: string, nodeId: string, toolName: string, cost: number): BuilderProject {
  checkedCost(cost);
  if (!validId(toolName)) fail('Invalid tool ID');
  const copy = addTransform(project, tenant, nodeId);
  const node = copy.nodes.find((n: GraphNode) => n.id === nodeId)!;
  if (copy.max_cost_minor > MAX_COST - cost) fail('Tool budget overflow');
  node.kind = 'tool'; node.tool_name = toolName; node.max_cost_minor = cost;
  copy.max_cost_minor += cost;
  reconcileGrants(copy);
  return canonicalProject(copy, tenant);
}
export function removeStep(project: BuilderProject, tenant: string, nodeId: string): BuilderProject {
  const copy = editCopy(project, tenant);
  const node = copy.nodes.find((n: GraphNode) => n.id === nodeId);
  const incoming = copy.edges.filter((e: GraphEdge) => e.to_node === nodeId);
  const outgoing = copy.edges.filter((e: GraphEdge) => e.from_node === nodeId);
  if (!node || (node.kind !== 'transform' && node.kind !== 'tool') ||
      node.inputs.length !== 1 || node.outputs.length !== 1 || incoming.length !== 1 || outgoing.length !== 1 ||
      node.inputs[0].kind !== node.outputs[0].kind) fail('NoCode conversion lossiness: removal requires one type-preserving input and output');
  const before = incoming[0]; const after = outgoing[0];
  copy.nodes = copy.nodes.filter((n: GraphNode) => n.id !== nodeId);
  copy.edges = copy.edges.filter((e: GraphEdge) => e.from_node !== nodeId && e.to_node !== nodeId);
  copy.edges.push({ from_node: before.from_node, from_port: before.from_port, to_node: after.to_node, to_port: after.to_port });
  copy.max_cost_minor -= node.max_cost_minor;
  reconcileGrants(copy);
  return canonicalProject(copy, tenant);
}
export function renameNode(project: BuilderProject, tenant: string, oldId: string, newId: string): BuilderProject {
  const copy = editCopy(project, tenant);
  const node = copy.nodes.find((n: GraphNode) => n.id === oldId);
  if (!node || !validId(newId) || copy.nodes.some((n: GraphNode) => n.id === newId)) fail('Invalid rename ID');
  node.id = newId;
  for (const edge of copy.edges) {
    if (edge.from_node === oldId) edge.from_node = newId;
    if (edge.to_node === oldId) edge.to_node = newId;
  }
  return canonicalProject(copy, tenant);
}
export function updateTool(project: BuilderProject, tenant: string, nodeId: string, toolName: string, cost: number): BuilderProject {
  checkedCost(cost);
  const copy = editCopy(project, tenant);
  const node = copy.nodes.find((n: GraphNode) => n.id === nodeId);
  if (!node || node.kind !== 'tool' || !validId(toolName)) fail('Invalid tool update');
  const remainder = copy.max_cost_minor - node.max_cost_minor;
  if (remainder > MAX_COST - cost) fail('Tool budget overflow');
  node.tool_name = toolName; node.max_cost_minor = cost;
  copy.max_cost_minor = remainder + cost;
  reconcileGrants(copy);
  return canonicalProject(copy, tenant);
}
