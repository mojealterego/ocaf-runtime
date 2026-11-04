'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function element(id) {
  return {
    id, value: '', hidden: false, textContent: '', firstChild: null, children: [],
    appendChild(child) { this.children.push(child); this.firstChild = this.children[0] || null; },
    removeChild(child) { this.children.splice(this.children.indexOf(child), 1); this.firstChild = this.children[0] || null; },
    click() { if (this.onclick) this.onclick(); }
  };
}
const ids = ['status','project','code-panel','nocode-panel','source','nodes','code','nocode','workspace','add','node-name','apply','save','load'];
const elements = Object.fromEntries(ids.map(id => [id, element(id)]));
const storage = new Map();
const document = {
  getElementById(id) { assert.ok(elements[id], id); return elements[id]; },
  createElement(tag) { return element(tag); },
  addEventListener(name, cb) { if (name === 'keydown') this.keydown = cb; }
};
const localStorage = {
  setItem(key, value) { storage.set(key, value); },
  getItem(key) { return storage.has(key) ? storage.get(key) : null; }
};
const script = fs.readFileSync(path.join(__dirname, '..', 'app.js'), 'utf8');
vm.runInNewContext(script, { document, localStorage, Array, JSON, Object, Error }, { timeout: 2000 });
function click(id) { elements[id].click(); }
function nodes() { return JSON.parse(elements.source.value).nodes.map(n => n.id); }

elements['node-name'].value = 'first';
click('add');
assert.deepEqual(nodes(), ['first']);
click('code');
elements.source.value = JSON.stringify({ version: 1, project_id: 'agent', nodes: [{id:'second'}] });
click('nocode');
assert.deepEqual(nodes(), ['second'], 'Code draft must survive mode switch');
click('code');
elements.source.value = JSON.stringify({ version: 1, project_id: 'agent', nodes: [{id:'third'}] });
click('save');
assert.deepEqual(JSON.parse(storage.get('ocaf-kaios-v1:agent')).nodes, [{id:'third'}], 'Code draft must be saved');
elements.source.value = '{broken';
click('nocode');
assert.equal(elements['code-panel'].hidden, false, 'Invalid Code draft must not change mode');
assert.match(elements.status.textContent, /JSON|Unexpected|property|token|position|end/i);
elements.source.value = JSON.stringify({ version: 1, project_id: 'agent', nodes: [{id:'third'},{id:'third'}] });
click('save');
assert.deepEqual(JSON.parse(storage.get('ocaf-kaios-v1:agent')).nodes, [{id:'third'}], 'Invalid draft must not overwrite storage');
elements.source.value = JSON.stringify({version:1,project_id:'agent',nodes:[{id:'third',unexpected:1}]});
click('apply');
assert.match(elements.status.textContent, /nieobsługiwany/);
elements.source.value = JSON.stringify({version:1,project_id:'agent',nodes:[],edges:[]});
click('apply');
assert.match(elements.status.textContent, /Nieobsługiwane pola projektu/);
elements.source.value = JSON.stringify({version:1,project_id:'agent',nodes:[{id:'toString'},{id:'constructor'}]});
click('apply');
assert.deepEqual(nodes(), ['toString','constructor'], 'Prototype-like node IDs must remain valid');
elements.source.value = ' '.repeat(16385);
click('apply');
assert.match(elements.status.textContent, /16 KiB/);
elements.source.value = JSON.stringify({version:1,project_id:'other',nodes:[]});
click('save');
assert.match(elements.status.textContent, /Niezgodny identyfikator projektu/);
elements.workspace.value = 'cloud';
elements.workspace.onchange();
assert.equal(elements.workspace.value, 'local', 'Cloud must fail closed');
elements.source.value = JSON.stringify({ version: 1, project_id: 'agent', nodes: [] });
click('nocode');
click('load');
assert.deepEqual(nodes(), ['third'], 'Stored project must reopen');
storage.set('ocaf-kaios-v1:agent', JSON.stringify({version:1,project_id:'wrong',nodes:[]}));
click('load');
assert.match(elements.status.textContent, /Niezgodny identyfikator zapisanego projektu/);
assert.deepEqual(nodes(), ['third']);
storage.set('ocaf-kaios-v1:agent', ' '.repeat(16385));
click('load');
assert.match(elements.status.textContent, /16 KiB/);
assert.deepEqual(nodes(), ['third']);
click('code');
elements['project'].value = 'other';
elements.source.value = JSON.stringify({version:1,project_id:'agent',nodes:[]});
click('save');
assert.match(elements.status.textContent, /Niezgodny identyfikator/);
elements['project'].value = 'agent';
elements.source.value = JSON.stringify({version:1,project_id:'agent',nodes:[{id:'unsaved'}]});
const previousWrite = localStorage.setItem;
localStorage.setItem = () => { throw Error('Storage unavailable'); };
click('save');
assert.match(elements.status.textContent, /Storage unavailable/);
localStorage.setItem = previousWrite;
click('code');
assert.deepEqual(nodes(), ['third'], 'Failed storage write must not commit Code draft');
assert.ok(document.keydown, 'Keypad shortcuts must be installed');
console.log('KaiOS runtime behavior tests passed');
