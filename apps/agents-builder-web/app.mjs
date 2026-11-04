import {
  newProject, insertNode, removeNode, setTool, validateProject,
  encodeDsl, decodeDsl, saveProject, loadProject, listRevisions, restoreRevision
} from './editor.mjs';
import {CloudWorkspaceClient, CloudApiError} from './cloud.mjs';
import {initIntegrations} from './integrations-ui.mjs';

const TENANT = 'local';
const el = id => document.getElementById(id);
const state = {
  project:null, saved:0, mode:'nocode', selectedNode:null, selectedEdge:0, dirty:false,
  workspace:'local', localSnapshot:null, cloudSnapshot:null,
  cloud:{tenant:null, head:null, pending:false}
};
let cloudClient = null;
let refreshIntegrations = () => {};
const currentTenant = () => state.workspace === 'cloud' ? state.cloud.tenant : TENANT;
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
  try { Promise.resolve(task()).catch(fail); } catch (error) { fail(error); }
};
function persist(next) {
  if (!state.project || next.project_id !== currentId() || next.tenant_id !== currentTenant()) {
    throw new Error('tenant: project identity cannot be changed');
  }
  if (state.workspace === 'cloud') {
    next.revision = (state.cloud.head?.revision ?? 0) + 1;
    validateProject(next, currentTenant());
    state.project = next;
    state.saved = next.revision;
    state.cloud.pending = true;
    state.dirty = false;
    notice('Szkic chmurowy nie został zapisany. Użyj „Zapisz w chmurze”.');
  } else {
    saveProject(localStorage, next, state.saved);
    state.project = next;
    state.saved = next.revision;
    state.dirty = false;
    notice('Saved offline revision ' + next.revision + '. No tools were executed.');
  }
  render();
}
function openProject(projectId) {
  if (state.workspace !== 'local') throw new Error('workspace: use cloud open');
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
  if (state.workspace !== 'local') throw new Error('workspace: use cloud new');
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
  try { order = validateProject(project, currentTenant()).order; }
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
  if (!state.project) return;
  const revisions = state.workspace === 'local'
    ? listRevisions(localStorage, TENANT, currentId())
    : [state.cloud.head?.revision].filter(Boolean);
  for (const revision of revisions) {
    const option = document.createElement('option');
    option.value = String(revision);
    option.textContent = 'Revision ' + revision;
    select.append(option);
  }
  select.value = String(state.workspace === 'cloud' ? (state.cloud.head?.revision ?? 0) : state.saved);
}
function render() {
  const hasProject = Boolean(state.project);
  el('project-title').textContent = hasProject ? currentId() : 'Brak projektu';
  el('revision-label').textContent = hasProject ? 'r' + state.saved + (state.cloud.pending && state.workspace === 'cloud' ? ' (szkic)' : '') : '—';
  el('workspace-local').setAttribute('aria-pressed', String(state.workspace === 'local'));
  el('workspace-cloud').setAttribute('aria-pressed', String(state.workspace === 'cloud'));
  el('cloud-controls').hidden = state.workspace !== 'cloud';
  el('cloud-status').textContent = cloudClient && state.cloud.tenant
    ? 'OAuth: sesja w pamięci · tenant ' + state.cloud.tenant + (state.cloud.pending ? ' · niezapisany szkic' : '')
    : 'Brak połączenia z uwierzytelnionym gateway. Żadne żądania nie są wysyłane.';
  el('cloud-open').disabled = !cloudClient;
  el('cloud-new').disabled = !cloudClient;
  el('cloud-save').disabled = !cloudClient || !hasProject || !state.cloud.pending;
  el('new-project').disabled = state.workspace !== 'local';
  el('open-project').disabled = state.workspace !== 'local';
  el('import').disabled = state.workspace !== 'local';
  for (const id of ['mode-nocode','mode-code','preview','export','add-transform','add-tool','remove-node','update-tool','set-budget','apply-code']) {
    el(id).disabled = !hasProject;
  }
  el('restore').disabled = !hasProject || state.workspace !== 'local';
  el('mode-nocode').setAttribute('aria-pressed', String(state.mode === 'nocode'));
  el('mode-code').setAttribute('aria-pressed', String(state.mode === 'code'));
  el('nocode-view').hidden = state.mode !== 'nocode';
  el('code-view').hidden = state.mode !== 'code';
  el('dsl').value = hasProject ? encodeDsl(state.project, currentTenant()) : '';
  el('cost-budget').value = hasProject ? String(state.project.max_cost_minor) : '';
  el('grant-list').textContent = hasProject && state.project.grants.length ? state.project.grants.join(', ') : 'No tool grants';
  renderGraph();
  renderHistory();
  refreshIntegrations();
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
async function preview() {
  if (state.dirty) throw new Error('unapplied Code DSL edits: apply or discard changes before validation');
  if (!state.project) throw new Error('workspace: select a project');
  if (state.workspace === 'cloud') {
    if (!cloudClient) throw new Error('oauth: connect first');
    const plan = await cloudClient.preview(currentTenant(), state.project);
    el('preview-output').textContent = [
      'CLOUD VALIDATION — authenticated Rust plan only; no tool execution.',
      'Graph: ' + plan.ordered_nodes.join(' → '),
      'Requested tools: ' + (plan.requested_tools.join(', ') || 'none'),
      'Maximum declared cost: ' + plan.max_cost_minor,
      'SHA-256: ' + plan.project_sha256
    ].join('\n');
    notice('Chmurowa walidacja zakończona. Nie uruchomiono agenta ani publikacji.');
    return;
  }
  const plan = validateProject(state.project, TENANT);
  const steps = plan.order.map((id,index) => String(index + 1) + '. ' + id);
  el('preview-output').textContent = [
    'DRY RUN — static validation only; no execution or network effects.',
    'Graph: ' + steps.join(' → '),
    'Requested tools: ' + (plan.tools.join(', ') || 'none'),
    'Declared cost: ' + plan.cost + ' / ' + state.project.max_cost_minor,
    'Project: ' + currentId() + ' r' + state.saved
  ].join('\n');
  notice('Graph validated locally. No tools were executed.');
}
function download() {
  if (state.dirty) throw new Error('unapplied Code DSL edits: apply or discard changes before export');
  if (!state.project) throw new Error('workspace: select a project');
  const text = encodeDsl(state.project, currentTenant());
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
  if (state.workspace !== 'local') throw new Error('workspace: import locally first');
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
function setWorkspace(workspace) {
  if (workspace === state.workspace) return;
  if (state.dirty && !window.confirm('Odrzucić niezastosowane zmiany Code DSL?')) return;
  if (state.workspace === 'local') {
    state.localSnapshot = {project:state.project, saved:state.saved};
    state.workspace = 'cloud';
    state.project = state.cloudSnapshot?.project ?? null;
    state.saved = state.cloudSnapshot?.saved ?? 0;
    state.cloud.head = state.cloudSnapshot?.head ?? null;
    state.cloud.pending = state.cloudSnapshot?.pending ?? false;
  } else {
    state.cloudSnapshot = {project:state.project, saved:state.saved, head:state.cloud.head, pending:state.cloud.pending};
    state.workspace = 'local';
    state.project = state.localSnapshot.project;
    state.saved = state.localSnapshot.saved;
  }
  state.selectedNode = null;
  state.selectedEdge = 0;
  state.dirty = false;
  el('preview-output').textContent = 'Wybierz „Weryfikuj” po otwarciu projektu.';
  render();
  notice(workspace === 'cloud' ? 'Tryb Cloud. Wymagane OAuth i uwierzytelniony gateway TLS.' : 'Tryb lokalny. Żadne dane nie są automatycznie synchronizowane.');
}
function requireCloud() {
  if (state.workspace !== 'cloud' || !cloudClient || !state.cloud.tenant) {
    throw new Error('oauth: połącz z chmurą przed operacją');
  }
}
function connectCloud() {
  if (state.cloud.pending && !window.confirm('Odrzucić niezapisany szkic chmurowy?')) return;
  const token = el('cloud-token').value.trim();
  const tenant = el('cloud-tenant').value.trim();
  const client = new CloudWorkspaceClient({token, origin:location.origin});
  // The tenant field is only a local expectation. The server must bind token
  // to a trusted TLS subject and reject cross-tenant requests.
  validateProject(newProject(tenant, 'connection-check'), tenant);
  cloudClient = client;
  state.cloud = {tenant, head:null, pending:false};
  state.cloudSnapshot = null;
  state.project = null;
  state.saved = 0;
  el('cloud-token').value = '';
  render();
  notice('Token OAuth przechowywany wyłącznie w pamięci. Otwórz projekt, aby zweryfikować dostęp na serwerze.');
}
function disconnectCloud() {
  if (state.cloud.pending && !window.confirm('Odrzucić niezapisany szkic chmurowy?')) return;
  cloudClient = null;
  state.cloud = {tenant:null, head:null, pending:false};
  state.cloudSnapshot = null;
  state.project = null;
  state.saved = 0;
  el('cloud-token').value = '';
  render();
  notice('Rozłączono chmurę. Lokalna przestrzeń robocza pozostaje bez zmian.');
}
async function openCloudProject() {
  requireCloud();
  if (state.cloud.pending && !window.confirm('Odrzucić niezapisany szkic chmurowy?')) return;
  const {project,head} = await cloudClient.load(state.cloud.tenant, el('project-id').value.trim());
  state.project = project;
  state.cloud.head = head;
  state.cloud.pending = false;
  state.saved = project.revision;
  state.selectedNode = null;
  state.selectedEdge = 0;
  render();
  notice('Otworzono uwierzytelniony projekt chmurowy r' + head.revision + '.');
}
async function newCloudProject() {
  requireCloud();
  if (state.cloud.pending && !window.confirm('Odrzucić niezapisany szkic chmurowy?')) return;
  const project = newProject(state.cloud.tenant, el('project-id').value.trim());
  try {
    await cloudClient.load(state.cloud.tenant, project.project_id);
    throw new Error('conflict: projekt już istnieje w chmurze; użyj Otwórz');
  } catch (error) {
    if (!(error instanceof CloudApiError && error.status === 404)) throw error;
  }
  state.project = project;
  state.saved = 1;
  state.cloud.head = null;
  state.cloud.pending = true;
  state.selectedNode = null;
  state.selectedEdge = 0;
  render();
  notice('Nowy szkic w pamięci. Projekt nie istnieje jeszcze na serwerze — użyj Zapisz w chmurze.');
}
async function saveCloudProject() {
  requireCloud();
  if (!state.project || !state.cloud.pending) throw new Error('workspace: brak zmian do zapisania');
  const head = await cloudClient.save(state.cloud.tenant, state.project, state.cloud.head);
  state.cloud.head = head;
  state.cloud.pending = false;
  state.saved = head.revision;
  render();
  notice('Serwer potwierdził zapis CAS r' + head.revision + '. SHA-256: ' + head.sha256);
}
function init() {
  el('workspace-local').addEventListener('click', () => safe(() => setWorkspace('local')));
  el('workspace-cloud').addEventListener('click', () => safe(() => setWorkspace('cloud')));
  el('cloud-connect').addEventListener('click', () => safe(connectCloud));
  el('cloud-disconnect').addEventListener('click', () => safe(disconnectCloud));
  el('cloud-open').addEventListener('click', () => safe(openCloudProject));
  el('cloud-new').addEventListener('click', () => safe(newCloudProject));
  el('cloud-save').addEventListener('click', () => safe(saveCloudProject));
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
    if (parsed.project_id !== currentId() || parsed.tenant_id !== currentTenant()) throw new Error('tenant: project identity cannot change');
    parsed.revision = state.saved + 1;
    const next = decodeDsl(JSON.stringify(parsed), currentTenant());
    persist(next);
  }));
  el('preview').addEventListener('click', () => safe(preview));
  el('export').addEventListener('click', () => safe(download));
  el('import').addEventListener('change', event => safe(() => importFile(event.target.files[0])));
  el('restore').addEventListener('click', () => safe(() => {
    if (state.workspace !== 'local') throw new Error('workspace: remote history restore not available');
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
  refreshIntegrations = initIntegrations({
    getContext: () => ({
      tenant: currentTenant(),
      projectId: state.project?.project_id ?? null,
      workspace: state.workspace
    }),
    notify: (message, isError=false) => isError ? fail(new Error(message)) : notice(message)
  });
  render();
  notice('Offline workspace ready. Code/NoCode, Local/Cloud and model profiles configured; no automatic execution.');
  if ('serviceWorker' in navigator && location.protocol.startsWith('http')) {
    navigator.serviceWorker.register('./sw.js').catch(() => {});
  }
}
init();
