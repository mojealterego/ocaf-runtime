import test from 'node:test';
import assert from 'node:assert/strict';
import {applyLanguage,translate,registerCatalog,plural,supportedLocales} from '../i18n.mjs';
import {ProviderSessionKeys,PROVIDERS} from '../credentials.mjs';
import {defaultSettings,validateSettings,listHuggingFaceGguf,huggingFaceDownloadUrl,
  parseGgufHeader,inspectLocalGguf,makeModelProfile,saveModelProfile,
  loadModelProfile,exportModelProfile,validateModelProfile} from '../models.mjs';

const storage=()=>{const map=new Map();return {
  getItem:key=>map.get(key)??null,setItem:(key,v)=>map.set(key,v),
  raw:()=>JSON.stringify([...map.entries()])
};};
test('Polish default, English/Czech/Slovak and RTL Arabic with safe custom catalogs',()=>{
  assert.equal(translate('pl-PL','title'),'Projektuj agentów.');
  assert.equal(translate('cs-CZ','title'),'Navrhujte agenty.');
  assert.equal(translate('sk-SK','title'),'Navrhujte agentov.');
  assert.equal(translate('en-US','title'),'Build agents.');
  assert.equal(translate('xx-XX','title'),'Projektuj agentów.');
  const nodes=[{dataset:{i18n:'title'},textContent:''},{dataset:{i18nPlaceholder:'hf_repo'},placeholder:''}];
  const document={documentElement:{},querySelectorAll:q=>q==='[data-i18n]'?[nodes[0]]:[nodes[1]]};
  assert.equal(applyLanguage(document,'ar-SA'),'rtl');
  assert.equal(document.documentElement.dir,'rtl');
  assert.equal(nodes[0].textContent,'أنشئ الوكلاء.');
  assert.equal(applyLanguage(document,'pl-PL'),'ltr');
  assert.equal(nodes[0].textContent,'Projektuj agentów.');
  assert.equal(nodes[1].placeholder,'Repozytorium Hugging Face');
  assert.equal(plural('pl-PL',3,{one:'a',few:'b',many:'c',other:'d'}),'b');
  registerCatalog('it-IT',{title:'Crea agenti.'});
  assert.equal(translate('it-IT','title'),'Crea agenti.');
  assert.ok(supportedLocales().some(x=>x[0]==='it-IT'));
  assert.throws(()=>registerCatalog('evil/../',{title:'x'}),/locale/);
  assert.throws(()=>registerCatalog('it-IT',{__secret:'x'}),/locale/);
});
test('API keys are private, memory-only, revocable and never included in model profile exports',()=>{
  const vault=new ProviderSessionKeys();
  assert.ok(PROVIDERS.length>=8);
  vault.add('openai','sk-test-secret');
  assert.equal(vault.has('openai'),true);
  assert.deepEqual(vault.identifiers(),['openai']);
  assert.equal(JSON.stringify(vault).includes('sk-test-secret'),false);
  assert.equal(vault.withCredential('openai',x=>x),'sk-test-secret');
  assert.throws(()=>vault.add('openai','bad\ninject'),/credential/);
  assert.throws(()=>vault.add('unknown','secret'),/provider/);
  vault.remove('openai');
  assert.equal(vault.has('openai'),false);
  assert.throws(()=>vault.withCredential('openai',()=>{}),/missing/);
});
test('Hugging Face GGUF discovery validates path, pinned commit, size and never attaches secrets',async()=>{
  const requests=[];
  const sha='a'.repeat(40);
  const fake=async(url,init)=>{
    requests.push([url,init]);
    return new Response(JSON.stringify({sha,siblings:[
      {rfilename:'Q4_K_M/my model.Q4_K_M.gguf',size:2024},
      {rfilename:'readme.md',size:10},{rfilename:'../escape.gguf',size:2}
    ]}),{status:200});
  };
  const catalog=await listHuggingFaceGguf('org/Model',{fetchImpl:fake});
  assert.equal(catalog.files.length,1);
  assert.equal(catalog.files[0].size_bytes,2024);
  assert.equal(catalog.files[0].download_url,
    'https://huggingface.co/org/Model/resolve/'+sha+'/Q4_K_M/my%20model.Q4_K_M.gguf?download=true');
  assert.equal(requests[0][0],'https://huggingface.co/api/models/org/Model?files_metadata=true');
  assert.equal(requests[0][1].credentials,'omit');
  assert.throws(()=>huggingFaceDownloadUrl('org/Model','main','x.gguf'),/invalid/);
  await assert.rejects(listHuggingFaceGguf('org/../../local',{fetchImpl:fake}),/invalid/);
  assert.equal(requests.length,1);
});
test('GGUF binary header is actually inspected, not trusted on filename',async()=>{
  const bytes=new ArrayBuffer(24);
  const dv=new DataView(bytes);
  for(let i=0;i<4;i++)dv.setUint8(i,'GGUF'.charCodeAt(i));
  dv.setUint32(4,3,true);dv.setBigUint64(8,24n,true);dv.setBigUint64(16,0n,true);
  const valid=parseGgufHeader(bytes);
  assert.equal(valid.version,3);
  assert.equal(valid.tensor_count,24);
  const file={name:'model.gguf',size:24,slice:()=>({arrayBuffer:async()=>bytes})};
  assert.equal((await inspectLocalGguf(file)).filename,'model.gguf');
  dv.setUint8(0,0);
  assert.throws(()=>parseGgufHeader(bytes),/magic/);
  await assert.rejects(inspectLocalGguf({name:'model.gguf',size:24,slice:()=>({arrayBuffer:async()=>bytes})}),/magic/);
  assert.throws(()=>parseGgufHeader(new ArrayBuffer(8)),/truncated/);
});
test('model profile settings are bounded, per project, exportable and schema-fail-closed',()=>{
  const settings=defaultSettings();
  assert.equal(settings.context_size,4096);
  const profile=makeModelProfile({
    kind:'huggingface',repo_id:'org/Model',commit_sha:'a'.repeat(40),
    path:'model.Q4_K_M.gguf',size_bytes:2048
  },settings);
  const db=storage();
  saveModelProfile(db,'tenant-a','hello',profile);
  assert.deepEqual(loadModelProfile(db,'tenant-a','hello'),profile);
  assert.equal(loadModelProfile(db,'tenant-b','hello'),null);
  assert.ok(exportModelProfile(profile).includes('context_size'));
  assert.equal(exportModelProfile(profile).includes('api_key'),false);
  assert.throws(()=>validateModelProfile({...profile,api_key:'secret'}),/unsupported/);
  assert.throws(()=>validateSettings({...settings,context_size:100}),/context_size/);
  assert.throws(()=>validateSettings({...settings,max_output_tokens:99999}),/max_output_tokens/);
  assert.throws(()=>makeModelProfile({kind:'file',filename:'../evil.gguf',size_bytes:30,gguf_version:3}),/invalid/);
  assert.throws(()=>makeModelProfile({kind:'api',provider:'openai',model_id:'bad\nkey'}),/invalid/);
  const api=makeModelProfile({kind:'api',provider:'openai',model_id:'gpt-4.1-mini'});
  assert.equal(api.source.provider,'openai');
});

test('Hugging Face discovery respects an already-aborted signal before fetch',async()=>{
  const controller=new AbortController();
  controller.abort();
  let called=false;
  await assert.rejects(listHuggingFaceGguf('org/Model',{
    signal:controller.signal,
    fetchImpl:async()=>{called=true;throw Error('unexpected network call');}
  }),/abort/i);
  assert.equal(called,false);
});
