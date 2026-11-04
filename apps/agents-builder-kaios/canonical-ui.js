/* Independent canonical typed DAG validator UI. Never executes tools. */
(function () {
  'use strict';
  var check = document.getElementById('canonical-check');
  var result = document.getElementById('canonical-result');
  check.onclick = function () {
    try {
      var source = document.getElementById('canonical-source').value;
      if (source.length > 131072) throw Error('Kanoniczny projekt przekracza limit 128 KiB');
      if (typeof OCAFCanonicalGraph === 'undefined') throw Error('Brak modułu walidatora DAG');
      var plan = OCAFCanonicalGraph.compile(JSON.parse(source), document.getElementById('canonical-tenant').value);
      result.textContent = JSON.stringify(plan, null, 2);
    } catch (error) {
      result.textContent = 'Błąd walidacji: ' + (error.message || 'Nieprawidłowy graf');
    }
  };
}());
