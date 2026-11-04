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
