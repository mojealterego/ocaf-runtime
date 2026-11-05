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
    if (typeof source !== 'string') throw Error('Nieprawidłowy format JSON');
    if (source.length > 131072) throw Error('Kanoniczny projekt przekracza limit 128 KiB');
    if (typeof OCAFCanonicalGraph === 'undefined') throw Error('Brak modułu walidatora DAG');
    var project = JSON.parse(source);
    if (project.project_id !== get('canonical-id').value) throw Error('Niezgodny identyfikator projektu DAG');
    var plan = OCAFCanonicalGraph.compile(project, get('canonical-tenant').value);
    return { project: project, plan: plan, source: source };
  }
  function preview() {
    var view = get('canonical-graph-preview');
    if (!view) return;
    var data = checked();
    var edges = data.project.edges.map(function (e) {
      return e.from_node + '.' + e.from_port + ' → ' + e.to_node + '.' + e.to_port;
    });
    view.textContent = edges.length ? edges.join(' | ') : 'Brak połączeń';
  }
  function safe(fn) {
    try {
      fn();
      preview();
    } catch (error) {
      result.textContent = 'Błąd walidacji: ' + (error.message || 'Nieprawidłowy graf');
      var view = get('canonical-graph-preview');
      if (view) view.textContent = 'Podgląd niedostępny — graf wymaga walidacji';
    }
  }
  function selectedPortKind() {
    var kind = get('canonical-port-kind').value;
    if (['text', 'json', 'bytes'].indexOf(kind) < 0) throw Error('Nieobsługiwany typ portu');
    return kind;
  }
  function port(name, kind) { return { name: name, kind: kind }; }
  function node(id, kind, portKind) {
    return { id: id, kind: kind, inputs: kind === 'input' ? [] : [port('in', portKind)],
      outputs: kind === 'output' ? [] : [port('out', portKind)],
      tool_name: null, max_cost_minor: 0 };
  }
  function edge(from, to) {
    return { from_node: from, from_port: 'out', to_node: to, to_port: 'in' };
  }
  get('canonical-new').onclick = function () {
    safe(function () {
      key();
      var portKind = selectedPortKind();
      var project = { schema_version: 1, tenant_id: get('canonical-tenant').value,
        project_id: get('canonical-id').value, revision: 1, max_cost_minor: 0,
        grants: [], nodes: [node('input','input',portKind), node('output','output',portKind)],
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
          ['text', 'json', 'bytes'].indexOf(output.inputs[0].kind) < 0)
        throw Error('Graf nieobsługiwany przez operację NoCode Transform');
      var portKind = output.inputs[0].kind;
      var predecessor = project.nodes.filter(function (n) { return n.id === incoming[0].from_node; })[0];
      var outgoingPort = predecessor.outputs.filter(function (p) { return p.name === 'out'; })[0];
      if (!outgoingPort || outgoingPort.kind !== portKind)
        throw Error('Transform wymaga portów text/out/in');
      var candidate = JSON.parse(JSON.stringify(project));
      if (candidate.revision >= 9007199254740991) throw Error('Przekroczono zakres rewizji');
      candidate.revision++;
      candidate.nodes.push(node(id,'transform',portKind));
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
  get('canonical-tool').onclick = function () {
    safe(function () {
      var project = checked().project;
      var id = get('canonical-node-id').value;
      var toolName = get('canonical-tool-name').value;
      var rawCost = get('canonical-tool-cost').value;
      if (!validId(id) || !validId(toolName) ||
          project.nodes.some(function (n) { return n.id === id; }))
        throw Error('Nieprawidłowy identyfikator węzła lub narzędzia');
      if (!/^[1-9][0-9]*$/.test(rawCost)) throw Error('Nieprawidłowy koszt narzędzia');
      var cost = Number(rawCost);
      if (!isFinite(cost) || cost > 1000000000000 || cost < 1)
        throw Error('Koszt poza zakresem');
      var output = project.nodes.filter(function (n) { return n.kind === 'output'; })[0];
      if (output.inputs.length !== 1 || output.inputs[0].name !== 'in')
        throw Error('Nieobsługiwany węzeł Output');
      var incoming = project.edges.filter(function (e) { return e.to_node === output.id; });
      if (incoming.length !== 1 || incoming[0].to_port !== 'in')
        throw Error('Nieobsługiwane połączenie Output');
      var predecessor = project.nodes.filter(function (n) { return n.id === incoming[0].from_node; })[0];
      var port = predecessor.outputs.filter(function (p) { return p.name === incoming[0].from_port; })[0];
      if (!port || port.kind !== output.inputs[0].kind)
        throw Error('Niezgodność typów portów');
      if (project.nodes.length >= 64 || project.revision >= 9007199254740991)
        throw Error('Limit węzłów lub rewizji');
      var candidate = JSON.parse(JSON.stringify(project));
      var tool = node(id, 'tool', port.kind);
      tool.tool_name = toolName;
      tool.max_cost_minor = cost;
      candidate.nodes.push(tool);
      candidate.edges = candidate.edges.filter(function (e) {
        return !(e.from_node === incoming[0].from_node && e.from_port === incoming[0].from_port &&
          e.to_node === incoming[0].to_node && e.to_port === incoming[0].to_port);
      });
      candidate.edges.push({
        from_node: incoming[0].from_node, from_port: incoming[0].from_port,
        to_node: id, to_port: 'in'
      });
      candidate.edges.push(edge(id, output.id));
      if (candidate.grants.indexOf(toolName) < 0) candidate.grants.push(toolName);
      if (candidate.max_cost_minor + cost > 1000000000000)
        throw Error('Przekroczono budżet projektu');
      candidate.max_cost_minor += cost;
      candidate.revision++;
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-remove-tool').onclick = function () {
    safe(function () {
      var project = checked().project;
      var id = get('canonical-node-id').value;
      var target = project.nodes.filter(function (n) { return n.id === id; })[0];
      if (!target || target.kind !== 'tool' || target.inputs.length !== 1 ||
          target.outputs.length !== 1 || target.inputs[0].name !== 'in' ||
          target.outputs[0].name !== 'out' ||
          target.inputs[0].kind !== target.outputs[0].kind)
        throw Error('Nieobsługiwany węzeł Tool');
      var incoming = project.edges.filter(function (e) { return e.to_node === id; });
      var outgoing = project.edges.filter(function (e) { return e.from_node === id; });
      if (incoming.length !== 1 || outgoing.length !== 1 ||
          incoming[0].to_port !== 'in' || outgoing[0].from_port !== 'out')
        throw Error('Tool wymaga pojedynczego wejścia i wyjścia');
      if (project.revision >= 9007199254740991)
        throw Error('Przekroczono zakres rewizji');
      var candidate = JSON.parse(JSON.stringify(project));
      candidate.nodes = candidate.nodes.filter(function (n) { return n.id !== id; });
      candidate.edges = candidate.edges.filter(function (e) {
        return e.to_node !== id && e.from_node !== id;
      });
      candidate.edges.push({
        from_node: incoming[0].from_node, from_port: incoming[0].from_port,
        to_node: outgoing[0].to_node, to_port: outgoing[0].to_port
      });
      if (!candidate.nodes.some(function (n) {
        return n.kind === 'tool' && n.tool_name === target.tool_name;
      })) {
        candidate.grants = candidate.grants.filter(function (g) { return g !== target.tool_name; });
      }
      candidate.max_cost_minor -= target.max_cost_minor;
      if (candidate.max_cost_minor < 0) throw Error('Niespójny budżet');
      candidate.revision++;
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-duplicate').onclick = function () {
    safe(function () {
      var project = checked().project;
      var sourceId = get('canonical-new-node-id').value;
      var newId = get('canonical-node-id').value;
      var source = project.nodes.filter(function (n) { return n.id === sourceId; })[0];
      if (!source || source.kind !== 'transform' || !validId(newId) ||
          project.nodes.some(function (n) { return n.id === newId; }))
        throw Error('Wymagany unikatowy ID i istniejący Transform');
      if (source.inputs.length !== 1 || source.outputs.length !== 1 ||
          source.inputs[0].name !== 'in' || source.outputs[0].name !== 'out' ||
          source.inputs[0].kind !== source.outputs[0].kind)
        throw Error('Nieobsługiwany Transform');
      var outgoing = project.edges.filter(function (e) { return e.from_node === sourceId; });
      if (outgoing.length !== 1 || outgoing[0].from_port !== 'out')
        throw Error('Transform wymaga jednego wyjścia');
      if (project.nodes.length >= 64 || project.revision >= 9007199254740991)
        throw Error('Limit węzłów lub rewizji');
      var candidate = JSON.parse(JSON.stringify(project));
      var duplicate = JSON.parse(JSON.stringify(source));
      duplicate.id = newId;
      candidate.nodes.push(duplicate);
      candidate.edges = candidate.edges.filter(function (e) {
        return !(e.from_node === outgoing[0].from_node && e.from_port === outgoing[0].from_port &&
          e.to_node === outgoing[0].to_node && e.to_port === outgoing[0].to_port);
      });
      candidate.edges.push({
        from_node: sourceId, from_port: 'out', to_node: newId, to_port: 'in'
      });
      candidate.edges.push({
        from_node: newId, from_port: 'out',
        to_node: outgoing[0].to_node, to_port: outgoing[0].to_port
      });
      candidate.revision++;
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-update-cost').onclick = function () {
    safe(function () {
      var project = checked().project;
      var id = get('canonical-node-id').value;
      var rawCost = get('canonical-tool-cost').value;
      if (!/^[1-9][0-9]*$/.test(rawCost)) throw Error('Nieprawidłowy koszt narzędzia');
      var cost = Number(rawCost);
      if (!isFinite(cost) || Math.floor(cost) !== cost || cost > 1000000000000)
        throw Error('Koszt poza zakresem');
      var target = project.nodes.filter(function (n) { return n.id === id; })[0];
      if (!target || target.kind !== 'tool') throw Error('Nie znaleziono węzła Tool');
      if (project.revision >= 9007199254740991) throw Error('Przekroczono zakres rewizji');
      var candidate = JSON.parse(JSON.stringify(project));
      var adjusted = candidate.max_cost_minor - target.max_cost_minor + cost;
      if (!isFinite(adjusted) || adjusted < 0 || adjusted > 1000000000000)
        throw Error('Przekroczono budżet projektu');
      candidate.max_cost_minor = adjusted;
      candidate.nodes.filter(function (n) { return n.id === id; })[0].max_cost_minor = cost;
      candidate.revision++;
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-retype').onclick = function () {
    safe(function () {
      var project = checked().project;
      var kind = selectedPortKind();
      if (project.revision >= 9007199254740991) throw Error('Przekroczono zakres rewizji');
      var candidate = JSON.parse(JSON.stringify(project));
      // Deliberately restricted to homogeneous, single-port linear graphs.
      candidate.nodes.forEach(function (n) {
        if (n.inputs.length > 1 || n.outputs.length > 1 ||
            (n.inputs.length && n.inputs[0].name !== 'in') ||
            (n.outputs.length && n.outputs[0].name !== 'out'))
          throw Error('Migracja NoCode wymaga grafu jednokanałowego');
        n.inputs.forEach(function (p) { p.kind = kind; });
        n.outputs.forEach(function (p) { p.kind = kind; });
      });
      candidate.revision++;
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-rename').onclick = function () {
    safe(function () {
      var project = checked().project;
      var oldId = get('canonical-node-id').value;
      var newId = get('canonical-new-node-id').value;
      if (!validId(newId) || !project.nodes.some(function (n) { return n.id === oldId; }) ||
          project.nodes.some(function (n) { return n.id === newId; }))
        throw Error('Nieprawidłowe lub zajęte ID węzła');
      if (project.revision >= 9007199254740991)
        throw Error('Przekroczono zakres rewizji');
      var candidate = JSON.parse(JSON.stringify(project));
      candidate.nodes.forEach(function (n) { if (n.id === oldId) n.id = newId; });
      candidate.edges.forEach(function (e) {
        if (e.from_node === oldId) e.from_node = newId;
        if (e.to_node === oldId) e.to_node = newId;
      });
      candidate.revision++;
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-remove').onclick = function () {
    safe(function () {
      var project = checked().project;
      var id = get('canonical-node-id').value;
      var target = project.nodes.filter(function (n) { return n.id === id; })[0];
      if (!target || target.kind !== 'transform' || target.inputs.length !== 1 ||
          target.outputs.length !== 1 || target.inputs[0].name !== 'in' ||
          target.outputs[0].name !== 'out' || target.inputs[0].kind !== target.outputs[0].kind)
        throw Error('Nieobsługiwany węzeł Transform');
      var incoming = project.edges.filter(function (e) { return e.to_node === id; });
      var outgoing = project.edges.filter(function (e) { return e.from_node === id; });
      if (incoming.length !== 1 || outgoing.length !== 1 ||
          incoming[0].to_port !== 'in' || outgoing[0].from_port !== 'out')
        throw Error('Transform wymaga jednego wejścia i wyjścia');
      var candidate = JSON.parse(JSON.stringify(project));
      if (candidate.revision >= 9007199254740991) throw Error('Przekroczono zakres rewizji');
      candidate.revision++;
      candidate.nodes = candidate.nodes.filter(function (n) { return n.id !== id; });
      candidate.edges = candidate.edges.filter(function (e) {
        return e.to_node !== id && e.from_node !== id;
      });
      candidate.edges.push({
        from_node: incoming[0].from_node, from_port: incoming[0].from_port,
        to_node: outgoing[0].to_node, to_port: outgoing[0].to_port
      });
      var plan = OCAFCanonicalGraph.compile(candidate, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(candidate, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
  };
  get('canonical-check').onclick = function () {
    safe(function () { result.textContent = JSON.stringify(checked().plan, null, 2); });
  };
  get('canonical-export').onclick = function () {
    safe(function () {
      var data = checked();
      get('canonical-import').value = JSON.stringify(data.project, null, 2);
      result.textContent = 'DAG gotowy do skopiowania z pola eksportu.';
    });
  };
  get('canonical-import-button').onclick = function () {
    safe(function () {
      var raw = get('canonical-import').value;
      if (typeof raw !== 'string') throw Error('Nieprawidłowy import');
      if (raw.length > 131072) throw Error('Import przekracza 128 KiB');
      var parsed = JSON.parse(raw);
      if (parsed.project_id !== get('canonical-id').value) throw Error('Niezgodny identyfikator projektu DAG');
      var plan = OCAFCanonicalGraph.compile(parsed, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(parsed, null, 2);
      result.textContent = JSON.stringify(plan, null, 2);
    });
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
      if (typeof raw !== 'string' || raw.length > 131072) throw Error('Zapisany DAG przekracza limit 128 KiB');
      var parsed = JSON.parse(raw);
      if (parsed.project_id !== get('canonical-id').value) throw Error('Niezgodny projekt w pamięci');
      var plan = OCAFCanonicalGraph.compile(parsed, get('canonical-tenant').value);
      get('canonical-source').value = JSON.stringify(parsed, null, 2);
      result.textContent = 'Otworzono kanoniczny DAG. Węzły: ' + plan.ordered_nodes.join(', ');
    });
  };
}());
