/* ES5-compatible offline subset for keypad-oriented KaiOS devices.
 * Not the full OCAF graph compiler. Never executes agent tools. */
(function () {
  'use strict';
  var $ = function (id) { return document.getElementById(id); };
  var state = { version: 1, project_id: 'agent', nodes: [] };
  var mode = 'nocode';
  function status(message) { $('status').textContent = message; }
  function validName(name) { return typeof name === 'string' && /^[a-zA-Z0-9_-]{1,48}$/.test(name); }
  function validate(value) {
    if (!value || value.version !== 1 || !validName(value.project_id) ||
        !Array.isArray(value.nodes) || value.nodes.length > 32) throw Error('Nieprawidłowy projekt');
    if (Object.keys(value).sort().join(',') !== 'nodes,project_id,version') throw Error('Nieobsługiwane pola projektu');
    var seen = Object.create(null);
    for (var i = 0; i < value.nodes.length; i++) {
      var n = value.nodes[i];
      if (!n || !validName(n.id) || Object.keys(n).join(',') !== 'id' || Object.prototype.hasOwnProperty.call(seen, n.id)) throw Error('Nieprawidłowy, nieobsługiwany lub powtórzony węzeł');
      seen[n.id] = true;
    }
    return { version: 1, project_id: value.project_id, nodes: value.nodes.map(function (n) { return { id: n.id }; }) };
  }
  function render() {
    $('project').value = state.project_id;
    $('code-panel').hidden = mode !== 'code';
    $('nocode-panel').hidden = mode !== 'nocode';
    $('source').value = JSON.stringify(state, null, 2);
    var list = $('nodes');
    while (list.firstChild) list.removeChild(list.firstChild);
    state.nodes.forEach(function (node) {
      var item = document.createElement('li');
      item.textContent = node.id;
      var remove = document.createElement('button');
      remove.type = 'button';
      remove.textContent = 'Usuń ' + node.id;
      remove.onclick = function () { safe(function () {
        var remaining = state.nodes.filter(function (entry) { return entry.id !== node.id; });
        state = validate({ version: 1, project_id: state.project_id, nodes: remaining });
        render(); status('Węzeł usunięty.');
      }); };
      item.appendChild(remove);
      list.appendChild(item);
    });
  }
  function safe(fn) {
    try { fn(); } catch (error) { status(error.message || 'Błąd operacji'); }
  }
  $('code').onclick = function () { if (mode !== 'code') { mode = 'code'; render(); } };
  $('nocode').onclick = function () { safe(function () { if (mode === 'code') applySource(); mode = 'nocode'; render(); }); };
  $('workspace').onchange = function () {
    if ($('workspace').value !== 'local') {
      $('workspace').value = 'local';
      status('Chmura niedostępna: brak uwierzytelnionego klienta KaiOS.');
    }
  };
  $('add').onclick = function () { safe(function () {
    var id = $('node-name').value.trim();
    var next = validate({ version: 1, project_id: state.project_id, nodes: state.nodes.concat([{ id: id }]) });
    state = next; $('node-name').value = ''; render(); status('Węzeł dodany. Nie uruchomiono narzędzi.');
  }); };
  function parseSource() {
    var source = $('source').value;
    if (source.length > 16384) throw Error('Projekt przekracza limit 16 KiB');
    return validate(JSON.parse(source));
  }
  function applySource() { state = parseSource(); }
  $('apply').onclick = function () { safe(function () {
    applySource();
    render(); status('Zaimportowano obsługiwany podzbiór JSON.');
  }); };
  $('save').onclick = function () { safe(function () {
    var requestedId = $('project').value.trim();
    var draft = mode === 'code' ? parseSource() : state;
    if (mode === 'code' && draft.project_id !== requestedId) throw Error('Niezgodny identyfikator projektu w edytorze Code');
    var candidate = validate({ version: draft.version, project_id: requestedId, nodes: draft.nodes });
    localStorage.setItem('ocaf-kaios-v1:' + candidate.project_id, JSON.stringify(candidate));
    state = candidate;
    render(); status('Zapisano lokalnie.');
  }); };
  $('load').onclick = function () { safe(function () {
    var id = $('project').value.trim();
    if (!validName(id)) throw Error('Nieprawidłowy identyfikator');
    var raw = localStorage.getItem('ocaf-kaios-v1:' + id);
    if (!raw) throw Error('Nie znaleziono projektu');
    if (raw.length > 16384) throw Error('Zapisany projekt przekracza limit 16 KiB');
    var loaded = validate(JSON.parse(raw));
    if (loaded.project_id !== id) throw Error('Niezgodny identyfikator zapisanego projektu');
    state = loaded; render(); status('Otworzono projekt.');
  }); };
  $('canonical-check').onclick = function () { safe(function () {
    var source = $('canonical-source').value;
    if (source.length > 131072) throw Error('Kanoniczny projekt przekracza limit 128 KiB');
    if (typeof OCAFCanonicalGraph === 'undefined') throw Error('Brak modułu walidatora DAG');
    var plan = OCAFCanonicalGraph.compile(JSON.parse(source), $('canonical-tenant').value);
    $('canonical-result').textContent = JSON.stringify(plan, null, 2);
    status('Kanoniczny DAG poprawny. Plan statyczny, bez wykonywania narzędzi.');
  }); };
  document.addEventListener('keydown', function (event) {
    if (event.key === 'SoftLeft') { event.preventDefault(); $('save').click(); }
    if (event.key === 'SoftRight') { event.preventDefault(); $('load').click(); }
  });
  render();
}());
