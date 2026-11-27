import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';

const source=readFileSync(new URL('../sw.js',import.meta.url),'utf8');
const scope='https://example.test/builder/';
function harness({hit='current',keys=['ocaf-builder-web-v12','unrelated']}={}) {
  const listeners=new Map(), calls=[];
  runInNewContext(source,{
    URL,
    self:{location:{origin:new URL(scope).origin},registration:{scope},
      addEventListener:(name,handler)=>listeners.set(name,handler),
      skipWaiting:()=>calls.push(['skipWaiting']),clients:{claim:()=>calls.push(['claim'])}},
    caches:{open:async name=>{calls.push(['open',name]);return {
      addAll:async assets=>calls.push(['addAll',Array.from(assets)]),
      match:async req=>{calls.push(['match',req.url]);return hit;}
    };},match:async()=>{calls.push(['globalMatch']);return 'stale';},
      keys:async()=>keys,delete:async name=>calls.push(['delete',name])},
    fetch:async req=>{calls.push(['fetch',req.url]);return 'network';}
  });
  const dispatch=(name,request)=>{let pending;listeners.get(name)({request,
    waitUntil:p=>{pending=p;},respondWith:p=>{pending=p;}});return pending;};
  return {calls,dispatch};
}

test('PWA release installs a new cache and retires only older builder caches',async()=>{
  const h=harness({keys:['ocaf-builder-web-v12','ocaf-builder-web-v13','unrelated']});
  await h.dispatch('install');await h.dispatch('activate');
  assert.deepEqual(h.calls[0],['open','ocaf-builder-web-v13']);
  assert.ok(h.calls[1][1].includes('./agent-catalog-ui.mjs'));
  assert.deepEqual(h.calls.filter(x=>x[0]==='delete'),[['delete','ocaf-builder-web-v12']]);
  assert.ok(h.calls.some(x=>x[0]==='claim'));
});

test('static catalog requests consult only the current cache, never an unrelated cache',async()=>{
  const h=harness();const url=scope+'agent-catalog-ui.mjs';
  assert.equal(await h.dispatch('fetch',{method:'GET',url}),'current');
  assert.deepEqual(h.calls,[['open','ocaf-builder-web-v13'],['match',url]]);
});

test('cache miss falls back to network; private, foreign and non-GET requests bypass caching',async()=>{
  const h=harness({hit:null});const url=scope+'agent-catalog.json';
  assert.equal(await h.dispatch('fetch',{method:'GET',url}),'network');
  assert.deepEqual(h.calls,[['open','ocaf-builder-web-v13'],['match',url],['fetch',url]]);
  assert.equal(h.dispatch('fetch',{method:'POST',url}),undefined);
  assert.equal(h.dispatch('fetch',{method:'GET',url:scope+'api/private'}),undefined);
  assert.equal(h.dispatch('fetch',{method:'GET',url:'https://foreign.test/agent-catalog.json'}),undefined);
  assert.equal(h.calls.length,3);
});
