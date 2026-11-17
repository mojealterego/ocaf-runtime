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
  const copy = JSON.parse(JSON.stringify(project)) as BuilderProject;
  const order = new Map<string, number>();
  compiled.ordered_nodes.forEach((id: string, index: number) => order.set(id, index));
  copy.nodes.sort((a: GraphNode, b: GraphNode) => order.get(a.id)! - order.get(b.id)!);
  for (const node of copy.nodes) {
    node.inputs.sort((a: Port, b: Port) => a.name.localeCompare(b.name));
    node.outputs.sort((a: Port, b: Port) => a.name.localeCompare(b.name));
  }
  copy.edges.sort((a: GraphEdge, b: GraphEdge) =>
    JSON.stringify([a.from_node, a.from_port, a.to_node, a.to_port]).localeCompare(
      JSON.stringify([b.from_node, b.from_port, b.to_node, b.to_port])));
  copy.grants.sort();
  return copy;
}
export function readCode(source: string, tenant: string): BuilderProject {
  if (typeof source !== 'string' || new TextEncoder().encode(source).length > MAX_BYTES) fail('DSL too large');
  let parsed: BuilderProject;
  try { parsed = JSON.parse(source) as BuilderProject; }
  catch (_error) { return fail('Invalid JSON Code DSL'); }
  return canonicalProject(parsed, tenant);
}
export function writeCode(project: BuilderProject, tenant: string): string {
  const source = JSON.stringify(canonicalProject(project, tenant), null, 2);
  if (new TextEncoder().encode(source).length > MAX_BYTES) fail('DSL too large');
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
