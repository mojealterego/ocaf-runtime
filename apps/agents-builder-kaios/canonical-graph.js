/* OCAF canonical graph static validation for KaiOS, ES5, no effects.
 * Port types, node kinds, tenant boundary and topological order follow src/builder_core.rs.
 * Not a cryptographic compiler or execution runtime. */
(function (root) {
  'use strict';
  function fail(message) { throw Error(message); }
  // ES5-compatible shared JSON admission boundary for the two KaiOS editors.
  function parseUniqueJson(source, maxBytes) {
    if (typeof source !== 'string') fail('JSON: expected text');
    if (maxBytes !== 16384 && maxBytes !== 131072) fail('JSON: invalid size limit');
    var bytes = 0, i, c;
    for (i = 0; i < source.length; i++) {
      c = source.charCodeAt(i);
      if (c <= 0x7f) bytes++;
      else if (c <= 0x7ff) bytes += 2;
      else if (c >= 0xd800 && c <= 0xdbff && i + 1 < source.length &&
          source.charCodeAt(i + 1) >= 0xdc00 && source.charCodeAt(i + 1) <= 0xdfff) {
        bytes += 4; i++;
      } else bytes += 3;
      if (bytes > maxBytes) fail('JSON: przekroczono limit ' + (maxBytes / 1024) + ' KiB');
    }
    // Reject ambiguous keys and excessive nesting before parsing the complete value.
    var frames = [];
    for (i = 0; i < source.length; i++) {
      c = source.charAt(i);
      if (c === '{' || c === '[') {
        if (frames.length >= 64) fail('JSON: nesting depth limit');
        frames.push({ object: c === '{', seen: Object.create(null) });
      } else if (c === '}' || c === ']') {
        frames.pop();
      } else if (c === '"') {
        var start = i;
        for (i++; i < source.length; i++) {
          c = source.charAt(i);
          if (c === '\\') { i++; continue; }
          if (c === '"') break;
        }
        var j = i + 1;
        while (j < source.length && /[ \t\r\n]/.test(source.charAt(j))) j++;
        var frame = frames[frames.length - 1];
        if (source.charAt(j) === ':' && frame && frame.object) {
          var key = JSON.parse(source.slice(start, i + 1));
          if (Object.prototype.hasOwnProperty.call(frame.seen, key)) fail('JSON: duplicate object key');
          frame.seen[key] = true;
        }
      }
    }
    return JSON.parse(source);
  }
  function safeInteger(n) { return typeof n === 'number' && isFinite(n) && Math.floor(n) === n && Math.abs(n) <= 9007199254740991; }
  function id(s) { return typeof s === 'string' && /^[A-Za-z0-9_.-]{1,64}$/.test(s) && s !== '.' && s !== '..'; }
  function keys(v, expected) {
    return v && typeof v === 'object' && !Array.isArray(v) &&
      Object.keys(v).length === expected.length && expected.every(function (key) {
        return Object.prototype.hasOwnProperty.call(v, key);
      });
  }
  function ports(items) {
    if (!Array.isArray(items) || items.length > 8) fail('Invalid ports');
    var seen = Object.create(null);
    items.forEach(function (p) {
      if (!keys(p, ['name', 'kind']) || !id(p.name) ||
          ['text', 'json', 'bytes'].indexOf(p.kind) < 0 || seen[p.name]) fail('Invalid port');
      seen[p.name] = true;
    });
  }
  function compile(p, trustedTenant) {
    if (!keys(p, ['schema_version','tenant_id','project_id','revision','max_cost_minor','grants','nodes','edges']))
      fail('Unsupported schema');
    if (!id(trustedTenant) || p.tenant_id !== trustedTenant) fail('Tenant denied');
    if (p.schema_version !== 1 || !id(p.project_id) || !safeInteger(p.revision) || p.revision < 1)
      fail('Invalid project');
    if (!safeInteger(p.max_cost_minor) || p.max_cost_minor < 0 || p.max_cost_minor > 1000000000000)
      fail('Invalid budget');
    if (!Array.isArray(p.nodes) || p.nodes.length < 2 || p.nodes.length > 64 ||
        !Array.isArray(p.edges) || p.edges.length > 256 ||
        !Array.isArray(p.grants) || p.grants.length > 32) fail('Resource limit');
    var grants = Object.create(null);
    p.grants.forEach(function (g) { if (!id(g) || grants[g]) fail('Invalid grant'); grants[g] = true; });
    var nodes = Object.create(null), indegree = Object.create(null), children = Object.create(null);
    var inputs = [], outputs = [], requested = Object.create(null), cost = 0;
    p.nodes.forEach(function (n) {
      if (!keys(n, ['id','kind','inputs','outputs','tool_name','max_cost_minor']) || !id(n.id) || nodes[n.id])
        fail('Invalid node');
      if (['input','output','transform','tool'].indexOf(n.kind) < 0) fail('Invalid kind');
      ports(n.inputs); ports(n.outputs);
      if (!safeInteger(n.max_cost_minor) || n.max_cost_minor < 0) fail('Invalid node cost');
      if (n.kind === 'input') {
        if (n.inputs.length || !n.outputs.length || n.tool_name !== null || n.max_cost_minor) fail('Invalid input');
        inputs.push(n.id);
      } else if (n.kind === 'output') {
        if (!n.inputs.length || n.outputs.length || n.tool_name !== null || n.max_cost_minor) fail('Invalid output');
        outputs.push(n.id);
      } else if (n.kind === 'transform') {
        if (!n.inputs.length || !n.outputs.length || n.tool_name !== null || n.max_cost_minor) fail('Invalid transform');
      } else {
        if (!n.inputs.length || !n.outputs.length || !id(n.tool_name) || n.max_cost_minor < 1) fail('Invalid tool');
        requested[n.tool_name] = true;
        cost += n.max_cost_minor;
        if (!safeInteger(cost)) fail('Cost overflow');
      }
      nodes[n.id] = n; indegree[n.id] = 0; children[n.id] = [];
    });
    if (inputs.length !== 1 || outputs.length !== 1) fail('Invalid endpoints');
    if (Object.keys(requested).sort().join(',') !== Object.keys(grants).sort().join(','))
      fail('Permission mismatch');
    if (cost > p.max_cost_minor) fail('Budget exceeded');
    var seenEdges = Object.create(null), occupied = Object.create(null);
    p.edges.forEach(function (e) {
      if (!keys(e, ['from_node','from_port','to_node','to_port']) || !id(e.from_node) || !id(e.to_node) || !id(e.from_port) || !id(e.to_port)) fail('Invalid edge');
      var a = nodes[e.from_node], b = nodes[e.to_node];
      if (!a || !b || e.from_node === e.to_node) fail('Invalid edge nodes');
      var from = a.outputs.filter(function (x) { return x.name === e.from_port; })[0];
      var to = b.inputs.filter(function (x) { return x.name === e.to_port; })[0];
      if (!from || !to) fail('Invalid edge port');
      if (from.kind !== to.kind) fail('Port type mismatch');
      var edgeKey = JSON.stringify([e.from_node,e.from_port,e.to_node,e.to_port]);
      var portKey = JSON.stringify([e.to_node,e.to_port]);
      if (seenEdges[edgeKey] || occupied[portKey]) fail('Duplicate edge or input');
      seenEdges[edgeKey] = true; occupied[portKey] = true;
      indegree[e.to_node]++; children[e.from_node].push(e.to_node);
    });
    p.nodes.forEach(function (n) {
      n.inputs.forEach(function (port) {
        if (!occupied[JSON.stringify([n.id,port.name])]) fail('Unconnected input');
      });
    });
    var ready = Object.keys(indegree).filter(function (n) { return indegree[n] === 0; }).sort();
    var ordered = [];
    while (ready.length) {
      var next = ready.shift(); ordered.push(next);
      children[next].forEach(function (child) {
        indegree[child]--;
        if (indegree[child] === 0) { ready.push(child); ready.sort(); }
      });
    }
    if (ordered.length !== p.nodes.length) fail('Cyclic graph');
    function reach(start, reverse) {
      var visited = Object.create(null), stack = [start];
      while (stack.length) {
        var current = stack.pop();
        if (visited[current]) continue;
        visited[current] = true;
        p.edges.forEach(function (e) {
          if (!reverse && e.from_node === current) stack.push(e.to_node);
          if (reverse && e.to_node === current) stack.push(e.from_node);
        });
      }
      return Object.keys(visited).length;
    }
    if (reach(inputs[0], false) !== p.nodes.length || reach(outputs[0], true) !== p.nodes.length)
      fail('Disconnected graph');
    return { ordered_nodes: ordered, requested_tools: Object.keys(requested).sort(),
      max_cost_minor: p.max_cost_minor };
  }
  root.OCAFCanonicalGraph = { compile: compile, parseUniqueJson: parseUniqueJson };
}(typeof module !== 'undefined' && module.exports ? module.exports : this));
