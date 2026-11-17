import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

// Exercise the actual handlers with a controlled Preferences adapter. This is
// not an ArkUI render, real filesystem flush, or device/simulator test.
const dir = mkdtempSync(join(tmpdir(),'ocaf-harmony-handlers-'));
process.on('exit',()=>rmSync(dir,{recursive:true,force:true}));
const root = new URL('../entry/src/main/ets/',import.meta.url);
const read = name => readFileSync(new URL(name,root),'utf8');
writeFileSync(join(dir,'BuilderDomain.mts'),read('services/BuilderDomain.ets'));
const store = read('services/ProjectStore.ets')
  .replace("import { preferences } from '@kit.ArkData';",'')
  .replace("import { common } from '@kit.AbilityKit';",'')
  .replace("import { BuilderProject, readCode, writeCode } from './BuilderDomain';",
    "import type { BuilderProject } from './BuilderDomain.mts';\nimport { readCode, writeCode } from './BuilderDomain.mts';")
  .replace('private readonly database: preferences.Preferences;', 'private readonly database: any;')
  .replace('constructor(context: common.UIAbilityContext)', 'constructor(context: any)')
  .replace("preferences.getPreferencesSync(context, { name: 'ocaf_builder_local_v1' })",'context');
writeFileSync(join(dir,'ProjectStore.mts'),store);
const pageSource = read('pages/BuilderPage.ets');
const marker = '\n  build() {';
assert.ok(pageSource.includes(marker));
const page = pageSource.slice(0,pageSource.indexOf(marker))
  .replace("import { common } from '@kit.AbilityKit';",'')
  .replace('  BuilderProject, addTransform', '  addTransform')
  .replace("} from '../services/BuilderDomain';", "} from './BuilderDomain.mts';\nimport type { BuilderProject } from './BuilderDomain.mts';")
  .replace("import { ProjectStore } from '../services/ProjectStore';","import { ProjectStore } from './ProjectStore.mts';")
  .replace('@Entry\n@Component\nstruct BuilderPage {','export class BuilderPage {\n  constructor(private readonly adapter: any) {}')
  .replace('constructor(private readonly adapter: any) {}','private readonly adapter: any;\n  constructor(adapter: any) { this.adapter = adapter; }')
  .replaceAll('@State ','')
  .replace('getContext(this) as common.UIAbilityContext','this.adapter') + '\n}\n';
writeFileSync(join(dir,'BuilderPage.mts'),page);
const {BuilderPage} = await import(pathToFileURL(join(dir,'BuilderPage.mts')).href);
const {ProjectStore} = await import(pathToFileURL(join(dir,'ProjectStore.mts')).href);
const core = await import(pathToFileURL(join(dir,'BuilderDomain.mts')).href);
function fixture(){
  const db=new Map(),calls=[];
  let finish,fail;
  const adapter={getSync:(key,defaultValue)=>db.has(key)?db.get(key):defaultValue,
    putSync:(key,value)=>{calls.push(key);db.set(key,value);},
    flush:()=>new Promise((resolve,reject)=>{finish=resolve;fail=reject;})};
  return {page:new BuilderPage(adapter),store:new ProjectStore(adapter),db,calls,finish:()=>finish(),fail:()=>fail(Error('flush failed'))};
}
test('actual NoCode handler applies tool, rename, update and removal to shared JSON',()=>{
  const {page}=fixture();page.nextNodeId='tool';page.toolCost='5';page.updateFromNoCode('tool');
  assert.equal(JSON.parse(page.source).max_cost_minor,5);
  page.renameTo='renamed';page.updateFromNoCode('rename');page.nextNodeId='renamed';page.toolCost='7';page.updateFromNoCode('update-tool');
  assert.equal(JSON.parse(page.source).max_cost_minor,7);
  page.updateFromNoCode('remove');assert.deepEqual(core.compileProject(JSON.parse(page.source),'local').ordered_nodes,['input','output']);
});
test('save is single-flight; pending flush prevents mutations and load',async()=>{
  const h=fixture(),p=h.page,before=p.source;
  const writing=p.save();assert.equal(p.saving,true);assert.equal(h.calls.length,1);
  await p.save();p.updateFromNoCode('transform');p.create();p.load();
  assert.equal(p.source,before);assert.equal(h.calls.length,1);assert.equal(p.saving,true);
  assert.doesNotMatch(p.status,/Zapisano projekt lokalnie/);
  h.finish();await writing;assert.equal(p.saving,false);assert.match(p.status,/Zapisano projekt lokalnie/);
});
test('failed flush releases exclusion without success claim',async()=>{
  const h=fixture(),before=h.page.source,pending=h.page.save();h.fail();await pending;
  assert.equal(h.page.saving,false);assert.equal(h.page.source,before);assert.match(h.page.status,/ODMOWA ZAPISU.*flush failed/);
});
test('wrong project ID and foreign tenant at stored key reject without replacing current graph',()=>{
  const h=fixture(),before=h.page.source,key='dag:local:my_agent';
  h.db.set(key,core.writeCode(core.newProject('local','other'),'local'));
  assert.throws(()=>h.store.load('local','my_agent'),/identity mismatch/);
  h.page.load();assert.equal(h.page.source,before);assert.match(h.page.status,/ODMOWA ODCZYTU/);
  h.db.set(key,core.writeCode(core.newProject('foreign','my_agent'),'foreign'));
  assert.throws(()=>h.store.load('local','my_agent'),/Tenant denied/);
});
test('cloud and identity changes fail closed before local mutation or storage',async()=>{
  const h=fixture(),p=h.page,before=p.source;p.cloudMode=true;
  p.updateFromNoCode('transform');p.create();p.load();await p.save();
  assert.equal(h.calls.length,0);assert.equal(p.source,before);
  p.cloudMode=false;p.projectId='other';p.updateFromNoCode('transform');await p.save();
  assert.equal(h.calls.length,0);assert.equal(p.source,before);
});
