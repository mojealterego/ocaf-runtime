'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const ids = ['status','project','code-panel','nocode-panel','source','nodes','code','nocode','workspace','add','node-name','apply','save','load'];
function element() {
  return {
    value: '', hidden: false, textContent: '', firstChild: null, children: [],
    appendChild(item) { this.children.push(item); this.firstChild = this.children[0]; },
    removeChild(item) { this.children.splice(this.children.indexOf(item), 1); this.firstChild = this.children[0] || null; },
    click() { if (this.onclick) this.onclick(); }
  };
}
const elements = Object.fromEntries(ids.map(id => [id, element()]));
let keydown;
const document = {
  getElementById(id) { return elements[id]; },
  createElement() { return element(); },
  addEventListener(event, handler) { if (event === 'keydown') keydown = handler; }
};
const storage = new Map();
const localStorage = {
  getItem(key) { return storage.get(key) || null; },
  setItem(key, value) { storage.set(key, value); }
};
const source = fs.readFileSync(path.join(__dirname, '..', 'app.js'), 'utf8');
vm.runInNewContext(source, { document, localStorage }, { timeout: 2000 });
assert.equal(typeof keydown, 'function');
let prevented = 0;
keydown({ key: 'SoftLeft', preventDefault() { prevented++; } });
assert.equal(prevented, 1, 'SoftLeft must suppress native default action');
assert.ok(storage.has('ocaf-kaios-v1:agent'), 'SoftLeft must save project');
keydown({ key: 'SoftRight', preventDefault() { prevented++; } });
assert.equal(prevented, 2, 'SoftRight must suppress native default action');
assert.match(elements.status.textContent, /Otworzono/, 'SoftRight must load project');
console.log('PASS KaiOS softkey action and default suppression');
