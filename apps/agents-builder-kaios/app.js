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
    var seen = {};
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
      list.appendChild(item);
    });
  }
  function safe(fn) {
    try { fn(); } catch (error) { status(error.message || 'Błąd operacji'); }
  }
  $('code').onclick = function () { mode = 'code'; render(); };
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
  function applySource() { state = validate(JSON.parse($('source').value)); }
  $('apply').onclick = function () { safe(function () {
    applySource();
    render(); status('Zaimportowano obsługiwany podzbiór JSON.');
  }); };
  $('save').onclick = function () { safe(function () {
    if (mode === 'code') applySource();
    state.project_id = $('project').value.trim();
    state = validate(state);
    localStorage.setItem('ocaf-kaios-v1:' + state.project_id, JSON.stringify(state));
    render(); status('Zapisano lokalnie.');
  }); };
  $('load').onclick = function () { safe(function () {
    var id = $('project').value.trim();
    if (!validName(id)) throw Error('Nieprawidłowy identyfikator');
    var raw = localStorage.getItem('ocaf-kaios-v1:' + id);
    if (!raw) throw Error('Nie znaleziono projektu');
    state = validate(JSON.parse(raw)); render(); status('Otworzono projekt.');
  }); };
  document.addEventListener('keydown', function (event) {
    if (event.key === 'SoftLeft') $('save').click();
    if (event.key === 'SoftRight') $('load').click();
  });
  render();
}());
