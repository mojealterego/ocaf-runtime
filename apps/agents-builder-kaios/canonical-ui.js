/* Offline canonical DAG editor boundary; no authentication or tool execution. */
(function () {
  'use strict';
  var get = function (id) { return document.getElementById(id); };
  var result = get('canonical-result');
  function validId(id) { return typeof id === 'string' && /^[A-Za-z0-9_.-]{1,64}$/.test(id); }
  function key() {
    var tenant = get('canonical-tenant').value;
    var project = get('canonical-id').value;
    if (!validId(tenant) || !validId(project)) throw Error('Nieprawidłowy tenant lub ID projektu');
    return 'ocaf-kaios-canonical-v1:' + tenant + ':' + project;
  }
  function checked() {
    var source = get('canonical-source').value;
    if (source.length > 131072) throw Error('Kanoniczny projekt przekracza limit 128 KiB');
    if (typeof OCAFCanonicalGraph === 'undefined') throw Error('Brak modułu walidatora DAG');
    var project = JSON.parse(source);
    if (project.project_id !== get('canonical-id').value) throw Error('Niezgodny identyfikator projektu DAG');
    var plan = OCAFCanonicalGraph.compile(project, get('canonical-tenant').value);
    return { project: project, plan: plan, source: source };
  }
  function safe(fn) {
    try { fn(); } catch (error) {
      result.textContent = 'Błąd walidacji: ' + (error.message || 'Nieprawidłowy graf');
    }
  }
  function port(name) { return { name: name, kind: 'text' }; }
  function node(id, kind) {
    return { id: id, kind: kind, inputs: kind === 'input' ? [] : [port('in')],
      outputs: kind === 'output' ? [] : [port('out')],
      tool_name: null, max_cost_minor: 0 };
  }
  function edge(from, to) {
    return { from_node: from, from_port: 'out', to_node: to, to_port: 'in' };
  }
  get('canonical-new').onclick = function () {
    safe(function () {
      key();
      var project = { schema_version: 1, tenant_id: get('canonical-tenant').value,
        project_id: get('canonical-id').value, revision: 1, max_cost_minor: 0,
        grants: [], nodes: [node('input','input'), node('output','output')],
        edges: [edge('input','output')] };
      var plan = OCAFCanonicalGraph.compile(project, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(project, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-transform').onclick = function () {
    safe(function () {
      var checkedProject = checked();
      var project = checkedProject.project;
      var id = get('canonical-node-id').value;
      if (!validId(id) || project.nodes.some(function (n) { return n.id === id; }))
        throw Error('Nieprawidłowy lub powtórzony identyfikator Transform');
      // Conservative NoCode operation: insert into the unique final edge.
      var output = project.nodes.filter(function (n) { return n.kind === 'output'; })[0];
      var incoming = project.edges.filter(function (e) { return e.to_node === output.id; });
      if (incoming.length !== 1 || output.inputs.length !== 1 ||
          incoming[0].to_port !== 'in' || incoming[0].from_port !== 'out' ||
          output.inputs[0].kind !== 'text')
        throw Error('Graf nieobsługiwany przez operację NoCode Transform');
      var predecessor = project.nodes.filter(function (n) { return n.id === incoming[0].from_node; })[0];
      var outgoingPort = predecessor.outputs.filter(function (p) { return p.name === 'out'; })[0];
      if (!outgoingPort || outgoingPort.kind !== 'text')
        throw Error('Transform wymaga portów text/out/in');
      var candidate = JSON.parse(JSON.stringify(project));
      candidate.nodes.push(node(id,'transform'));
      candidate.edges = candidate.edges.filter(function (e) {
        return !(e.from_node === incoming[0].from_node && e.from_port === incoming[0].from_port &&
          e.to_node === incoming[0].to_node && e.to_port === incoming[0].to_port);
      });
      candidate.edges.push(edge(predecessor.id,id));
      candidate.edges.push(edge(id,output.id));
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-check').onclick = function () {
    safe(function () { result.textContent = JSON.stringify(checked().plan, null, 2); });
  };
  get('canonical-save').onclick = function () {
    safe(function () {
      var data = checked();
      localStorage.setItem(key(), JSON.stringify(data.project));
      result.textContent = 'Zapisano kanoniczny DAG lokalnie.';
    });
  };
  get('canonical-load').onclick = function () {
    safe(function () {
      var raw = localStorage.getItem(key());
      if (!raw) throw Error('Nie znaleziono kanonicznego DAG');
      if (raw.length > 131072) throw Error('Zapisany DAG przekracza limit 128 KiB');
      var parsed = JSON.parse(raw);
      if (parsed.project_id !== get('canonical-id').value) throw Error('Niezgodny projekt w pamięci');
      var plan = OCAFCanonicalGraph.compile(parsed, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(parsed, null, 2);
      result.textContent = 'Otworzono kanoniczny DAG. Węzły: ' + plan.ordered_nodes.join(', ');
    });
  };
}());
