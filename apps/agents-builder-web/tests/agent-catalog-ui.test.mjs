import test from 'node:test';
import assert from 'node:assert/strict';
import {initPublicCatalog} from '../agent-catalog-ui.mjs';

class Element {
  constructor(){this.textContent='';this.value='';this.children=[];this.handlers=new Map();}
  addEventListener(name,handler){this.handlers.set(name,handler);}
  replaceChildren(...elements){this.children=[...elements];}
  append(...elements){this.children.push(...elements);}
  click(){this.handlers.get('click')?.();}
  dispatch(name){this.handlers.get(name)?.();}
  set innerHTML(_){throw Error('Unsafe innerHTML: untrusted source');}
}
const fixture=()=>({schema_version:1,provenance:'GIT_SOURCE_INVENTORY_ONLY_NOT_EXECUTION_AUTHORITY',
  fleet_id:'OCAF-FLEET-111',active_runtime_agent_count:0,
  source_agent_registry_git_blob:'a'.repeat(40),source_swarm_registry_git_blob:'b'.repeat(40),
  agents:Array.from({length:111},(_,i)=>({id:`OCAF-AGENT-${String(i+1).padStart(3,'0')}`,
    display_name:i===0?'<img src=x onerror=alert(1)>':'Agent '+(i+1),
    group_id:`G${String(Math.min(i+1,33)).padStart(2,'0')}`,specialization:'test',
    lifecycle:'DRAFT',deployment_status:'NOT_DEPLOYED',artifact_status:'NOT_BUILT',
    documentation_status:'NOT_VERIFIED',certification_status:'NOT_ISSUED',
    price_status:'NOT_PRICED',executable:false})),
  swarms:Array.from({length:33},(_,i)=>({id:`G${i+1}`,specialization:'test',scope:'Task',
    lifecycle:'REFERENCE_ONLY',deployment_status:'NOT_DEPLOYED',
    documentation_status:'NOT_VERIFIED',certification_status:'NOT_ISSUED',
    price_status:'NOT_PRICED',executable:false}))});
const doc=()=>{const map=new Map();return {
  getElementById:id=>{if(!map.has(id))map.set(id,new Element());return map.get(id);},
  createElement:()=>new Element()
};};

test('read-only catalog renders actual names as text, shows status without selling or executing',async()=>{
  const document=doc();
  await initPublicCatalog({document,fetchImpl:async()=>new Response(JSON.stringify(fixture()),
    {headers:{'content-type':'application/json'}})});
  const list=document.getElementById('catalog-list');
  assert.equal(list.children.length,30);
  assert.match(list.children[0].children[0].textContent,/img src=x/);
  list.children[0].children[0].click();
  const rows=document.getElementById('catalog-detail').children[0].children;
  assert.ok(rows.some(row=>row.textContent==='Certyfikat niewydany'));
  assert.ok(rows.some(row=>row.textContent==='Brak potwierdzonej ceny'));
  assert.ok(rows.some(row=>row.textContent.includes('ZABLOKOWANA')));
  assert.match(document.getElementById('catalog-status').textContent,/0 aktywnych/);
  const search=document.getElementById('catalog-query');search.value='Agent 111';search.dispatch('input');
  assert.equal(list.children.length,1);
});

test('network failure leaves no displayed data and clearly refuses access',async()=>{
  const document=doc();
  await assert.rejects(initPublicCatalog({document,fetchImpl:async()=>new Response('denied',
    {status:503,headers:{'content-type':'application/json'}})}),/catalog:/);
  assert.match(document.getElementById('catalog-status').textContent,/niedostępny/);
  assert.equal(document.getElementById('catalog-list').children.length,0);
});
