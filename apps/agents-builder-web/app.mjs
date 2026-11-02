import {
  newProject, insertNode, removeNode, setTool, validateProject,
  encodeDsl, decodeDsl, saveProject, loadProject, listRevisions, restoreRevision
} from './editor.mjs';

const TENANT = 'local';
const el = id => document.getElementById(id);
const state = {project:null, saved:0, mode:'nocode', selectedNode:null, selectedEdge:0, dirty:false};
const fail = error => {
  el('message').textContent = error instanceof Error ? error.message : String(error);
  el('message').dataset.level = 'error';
};
const notice = message => {
  el('message').textContent = message;
  el('message').dataset.level = 'info';
};
const setMode = mode => {
  if (state.mode === 'code' && state.dirty && !window.confirm('Discard unapplied Code DSL edits?')) return;
  state.mode = mode;
  state.dirty = false;
  render();
};
const currentId = () => state.project?.project_id ?? 'demo';
const safe = task => {
  try { task(); } catch (error) { fail(error); }
};
function persist(next) {
  if (next.project_id !== currentId() || next.tenant_id !== TENANT) throw new Error('tenant: project identity cannot be changed');
  saveProject(localStorage, next, state.saved);
  state.project = next;
  state.saved = next.revision;
  state.dirty = false;
  notice('Saved offline revision ' + next.revision + '. No tools were executed.');
  render();
}
function openProject(projectId) {
  const existing = loadProject(localStorage, TENANT, projectId);
  if (!existing) throw new Error('Project not found in this browser. Use New to create it.');
  state.project = existing;
  state.saved = existing.revision;
  state.selectedNode = null;
  state.selectedEdge = 0;
  state.dirty = false;
  notice('Opened local project ' + projectId + '.');
  render();
}
function makeProject(projectId) {
  const next = newProject(TENANT, projectId);
  if (loadProject(localStorage, TENANT, projectId)) throw new Error('Project already exists; open it instead.');
  saveProject(localStorage, next, 0);
  state.project = next;
  state.saved = next.revision;
  state.selectedNode = null;
  state.selectedEdge = 0;
  state.dirty = false;
  notice('New project created in browser-local storage.');
  render();
}
function nodeCard(node, position) {
  const card = document.createElement('button');
  card.type = 'button';
  card.className = 'node-card';
  card.dataset.kind = node.kind;
  card.dataset.selected = String(state.selectedNode === node.id);
  card.setAttribute('aria-pressed', String(state.selectedNode === node.id));
  const header = document.createElement('strong');
  header.textContent = node.id;
  const type = document.createElement('span');
  type.textContent = node.kind;
  const ports = document.createElement('small');
  ports.textContent = node.inputs.map(p => p.name + ':' + p.kind).join(', ') + ' → ' + node.outputs.map(p => p.name + ':' + p.kind).join(', ');
  card.append(header, type, ports);
  if (node.kind === 'tool') {
    const grant = document.createElement('small');
    grant.textContent = 'Grant: ' + node.tool_name + ' · max ' + node.max_cost_minor;
    card.append(grant);
  }
  card.style.setProperty('--order', String(position));
  card.addEventListener('click', () => {
    state.selectedNode = node.id;
    renderGraph();
  });
  return card;
}
function renderGraph() {
  const project = state.project;
  const graph = el('graph');
  const edges = el('edges');
  graph.replaceChildren();
  edges.replaceChildren();
  if (!project) return;
  let order;
  try { order = validateProject(project, TENANT).order; }
  catch (error) { fail(error); return; }
  for (const [index, name] of order.entries()) {
    graph.append(nodeCard(project.nodes.find(n => n.id === name), index));
  }
  for (const [index, edge] of project.edges.entries()) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'edge-button';
    button.dataset.selected = String(state.selectedEdge === index);
    button.setAttribute('aria-pressed', String(state.selectedEdge === index));
    button.textContent = edge.from_node + '.' + edge.from_port + ' → ' + edge.to_node + '.' + edge.to_port;
    button.addEventListener('click', () => {
      state.selectedEdge = index;
      renderGraph();
    });
    edges.append(button);
  }
  const selected = project.nodes.find(n => n.id === state.selectedNode);
  el('node-settings').hidden = selected?.kind !== 'tool';
  if (selected?.kind === 'tool') {
    el('tool-edit-name').value = selected.tool_name;
    el('tool-edit-cost').value = String(selected.max_cost_minor);
  }
  el('remove-node').disabled = !selected || !['tool','transform'].includes(selected.kind);
}
function renderHistory() {
  const select = el('revision');
  select.replaceChildren();
  for (const revision of listRevisions(localStorage, TENANT, currentId())) {
    const option = document.createElement('option');
    option.value = String(revision);
    option.textContent = 'Revision ' + revision;
    select.append(option);
  }
  select.value = String(state.saved);
}
function render() {
  if (!state.project) return;
  el('project-title').textContent = state.project.project_id;
  el('revision-label').textContent = 'r' + state.saved;
  el('mode-nocode').setAttribute('aria-pressed', String(state.mode === 'nocode'));
  el('mode-code').setAttribute('aria-pressed', String(state.mode === 'code'));
  el('nocode-view').hidden = state.mode !== 'nocode';
  el('code-view').hidden = state.mode !== 'code';
  el('dsl').value = encodeDsl(state.project, TENANT);
  el('cost-budget').value = String(state.project.max_cost_minor);
  el('grant-list').textContent = state.project.grants.length ? state.project.grants.join(', ') : 'No tool grants';
  renderGraph();
  renderHistory();
}
function add(kind) {
  const project = state.project;
  const nodeId = el('node-id').value.trim();
  const options = {edgeIndex:state.selectedEdge};
  if (kind === 'tool') {
    options.tool_name = el('tool-name').value.trim();
    options.max_cost_minor = Number(el('tool-cost').value);
  }
  persist(insertNode(project, kind, nodeId, options));
  state.selectedNode = nodeId;
  el('node-id').value = '';
  renderGraph();
}
function preview() {
  const plan = validateProject(state.project, TENANT);
  const steps = plan.order.map((id,index) => String(index + 1) + '. ' + id);
  el('preview-output').textContent = [
    'DRY RUN — static validation only; no execution or network effects.',
    'Graph: ' + steps.join(' → '),
    'Requested tools: ' + (plan.tools.join(', ') || 'none'),
    'Declared cost: ' + plan.cost + ' / ' + state.project.max_cost_minor,
    'Project: ' + currentId() + ' r' + state.saved
  ].join('\n');
  notice('Graph validated locally. Cloud admission and sandbox execution are not connected.');
}
function download() {
  const text = encodeDsl(state.project, TENANT);
  const blob = new Blob([text], {type:'application/json'});
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = currentId() + '.ocaf-builder.json';
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  notice('Exported bounded JSON DSL. Export is not signed or published.');
}
function importFile(file) {
  if (!file) return;
  if (file.size > 128 * 1024) throw new Error('limit: import exceeds 128 KiB');
  file.text().then(source => safe(() => {
    const project = decodeDsl(source, TENANT);
    if (loadProject(localStorage, TENANT, project.project_id)) throw new Error('conflict: import ID already exists; no overwrite');
    project.revision = 1;
    saveProject(localStorage, project, 0);
    state.project = project;
    state.saved = 1;
    state.selectedNode = null;
    state.selectedEdge = 0;
    notice('Imported project into local offline workspace.');
    render();
  })).catch(fail);
}
function init() {
  el('mode-code').addEventListener('click', () => setMode('code'));
  el('mode-nocode').addEventListener('click', () => setMode('nocode'));
  el('new-project').addEventListener('click', () => safe(() => makeProject(el('project-id').value.trim())));
  el('open-project').addEventListener('click', () => safe(() => openProject(el('project-id').value.trim())));
  el('add-transform').addEventListener('click', () => safe(() => add('transform')));
  el('add-tool').addEventListener('click', () => safe(() => add('tool')));
  el('remove-node').addEventListener('click', () => safe(() => {
    persist(removeNode(state.project, state.selectedNode));
    state.selectedNode = null;
    renderGraph();
  }));
  el('update-tool').addEventListener('click', () => safe(() => {
    persist(setTool(state.project, state.selectedNode, el('tool-edit-name').value.trim(), Number(el('tool-edit-cost').value)));
  }));
  el('set-budget').addEventListener('click', () => safe(() => {
    const next = structuredClone(state.project);
    next.max_cost_minor = Number(el('cost-budget').value);
    next.revision += 1;
    persist(next);
  }));
  el('dsl').addEventListener('input', () => { state.dirty = true; el('message').textContent = 'Unapplied Code DSL changes'; });
  el('apply-code').addEventListener('click', () => safe(() => {
    const raw = el('dsl').value;
    if (new TextEncoder().encode(raw).length > 128 * 1024) throw new Error('limit: DSL too large');
    const parsed = JSON.parse(raw);
    if (parsed.project_id !== currentId() || parsed.tenant_id !== TENANT) throw new Error('tenant: project identity cannot change');
    parsed.revision = state.saved + 1;
    const next = decodeDsl(JSON.stringify(parsed), TENANT);
    persist(next);
  }));
  el('preview').addEventListener('click', () => safe(preview));
  el('export').addEventListener('click', () => safe(download));
  el('import').addEventListener('change', event => safe(() => importFile(event.target.files[0])));
  el('restore').addEventListener('click', () => safe(() => {
    const revision = Number(el('revision').value);
    if (!window.confirm('Restore revision ' + revision + ' as a new revision?')) return;
    const restored = restoreRevision(localStorage, TENANT, currentId(), revision);
    state.project = restored;
    state.saved = restored.revision;
    notice('Restored revision ' + revision + ' as revision ' + state.saved + '.');
    render();
  }));
  for (const button of document.querySelectorAll('[data-palette]')) {
    button.draggable = true;
    button.addEventListener('dragstart', event => event.dataTransfer.setData('text/plain', button.dataset.palette));
  }
  const graph = el('graph');
  graph.addEventListener('dragover', event => { if (event.dataTransfer.types.includes('text/plain')) event.preventDefault(); });
  graph.addEventListener('drop', event => {
    event.preventDefault();
    const kind = event.dataTransfer.getData('text/plain');
    if (kind === 'tool' || kind === 'transform') safe(() => add(kind));
  });
  const previous = loadProject(localStorage, TENANT, 'demo');
  if (previous) {
    state.project = previous;
    state.saved = previous.revision;
  } else {
    const project = newProject(TENANT, 'demo');
    saveProject(localStorage, project, 0);
    state.project = project;
    state.saved = 1;
  }
  render();
  notice('Offline workspace ready. Select Code or NoCode; no remote operations.');
  if ('serviceWorker' in navigator && location.protocol.startsWith('http')) {
    navigator.serviceWorker.register('./sw.js').catch(() => {});
  }
}
init();
