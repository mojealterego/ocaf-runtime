'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
function element() {
  return {
    value: '', hidden: false, textContent: '', children: [], firstChild: null,
    appendChild(child) { this.children.push(child); this.firstChild = this.children[0]; },
    removeChild(child) { this.children.splice(this.children.indexOf(child), 1); this.firstChild = this.children[0] || null; },
    click() { if (this.onclick) this.onclick(); }
  };
}
const ids = ['status','project','code-panel','nocode-panel','source','nodes','code','nocode','workspace','add','node-name','apply','save','load'];
const elements = Object.fromEntries(ids.map(id => [id, element()]));
const document = {
  getElementById(id) { return elements[id]; },
  createElement() { return element(); },
  addEventListener() {}
};
const localStorage = { setItem() {}, getItem() { return null; } };
vm.runInNewContext(fs.readFileSync(path.join(__dirname, '..', 'app.js'), 'utf8'), { document, localStorage }, { timeout: 2000 });
elements['node-name'].value = 'first';
elements.add.click();
assert.equal(elements.nodes.children.length, 1);
const remove = elements.nodes.children[0].children[0];
assert.ok(remove, 'NoCode must expose a focusable remove button');
remove.click();
assert.deepEqual(JSON.parse(elements.source.value).nodes, [], 'Node removal must update shared Code/NoCode state');
assert.match(elements.status.textContent, /usunięty/);
console.log('PASS KaiOS NoCode node removal');
