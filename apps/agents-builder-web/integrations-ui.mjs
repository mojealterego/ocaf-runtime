import {LOCALES,supportedLocales,translate,applyLanguage,registerCatalog} from './i18n.mjs';
import {LocalRunnerClient} from './runner-client.mjs';
import {ProviderSessionKeys,PROVIDERS} from './credentials.mjs';
import {SETTINGS_SPEC,defaultSettings,validateSettings,makeModelProfile,
  listHuggingFaceGguf,inspectLocalGguf,saveModelProfile,loadModelProfile,
  exportModelProfile} from './models.mjs';

/** Pluggable UI for a separate, bounded, non-secret inference profile.
 * Inference and provider billing are never initiated from this component.
 */
export function initIntegrations({getContext,notify}){
  const el=id=>document.getElementById(id);
  const vault=new ProviderSessionKeys();
  let catalog=null,localGguf=null,contextKey=null,activeProfile=null;
  let runnerClient=null;
  let locale='pl-PL';
  try{
    const saved=localStorage.getItem('ocaf.builder.ui.locale');
    if(saved&&supportedLocales().some(([tag])=>tag===saved))locale=saved;
  }catch{}
  const emit=(message,error=false)=>{
    el('model-status').textContent=message;
    el('model-status').dataset.level=error?'error':'info';
    notify(message,error);
  };
  const safe=action=>event=>{
    try{Promise.resolve(action(event)).catch(error=>emit(error.message,true));}
    catch(error){emit(error.message,true);}
  };
  const context=()=>{
    const {tenant,projectId,workspace}=getContext();
    if(!tenant||!projectId)throw Error('profile: najpierw wybierz projekt');
    return {tenant,projectId,workspace};
  };
  function updateLanguage(){
    const select=el('ui-language');
    select.replaceChildren();
    for(const [tag,title] of supportedLocales()){
      const option=document.createElement('option');
      option.value=tag;option.textContent=title+' ('+tag+')';
      select.append(option);
    }
    select.value=locale;
    applyLanguage(document,locale);
  }
  function setLocale(tag){
    if(!supportedLocales().some(([id])=>id===tag))throw Error('locale: unsupported language');
    locale=tag;
    try{localStorage.setItem('ocaf.builder.ui.locale',tag);}catch{}
    updateLanguage();
  }
  async function importLocale(event){
    const file=event.target.files?.[0];
    if(!file)return;
    try{
      if(file.size>64*1024)throw Error('locale: catalog exceeds 64 KiB');
      const payload=JSON.parse(await file.text());
      if(!payload||Object.keys(payload).sort().join(',')!=='locale,messages')throw Error('locale: expected {locale,messages}');
      registerCatalog(payload.locale,payload.messages);
      setLocale(payload.locale);
    }finally{event.target.value='';}
  }
  function initProviders(){
    for(const id of ['api-provider','model-provider']){
      const select=el(id);
      for(const [value,title] of PROVIDERS){
        const option=document.createElement('option');
        option.value=value;option.textContent=title;select.append(option);
      }
    }
    el('api-provider').addEventListener('change',renderCredentials);
    el('api-add').addEventListener('click',safe(()=>{
      const key=el('api-secret').value;
      try{vault.add(el('api-provider').value,key);}
      finally{el('api-secret').value='';}
      renderCredentials();
    }));
    el('api-remove').addEventListener('click',safe(()=>{
      vault.remove(el('api-provider').value);
      el('api-secret').value='';
      renderCredentials();
    }));
    window.addEventListener('pagehide',()=>vault.clear(),{once:true});
    renderCredentials();
  }
  function renderCredentials(){
    const selected=el('api-provider').value;
    const status=vault.has(selected)
      ? 'Klucz '+selected+' skonfigurowany w pamięci tej karty; nie jest eksportowany.'
      : 'Brak klucza '+selected+' w pamięci. Nie skonfigurowano jeszcze brokera wywołań API.';
    el('api-status').textContent=status;
    el('api-secret').value='';
  }
  const settingNames={
    context_size:'ctx',threads:'threads',batch_size:'batch',gpu_layers:'gpu_layers',
    temperature:'temp',top_p:'top_p',top_k:'top_k',min_p:'min_p',
    repeat_penalty:'repeat',seed:'seed',max_output_tokens:'max_tokens',
    flash_attention:'flash',use_mmap:'mmap'
  };
  function initSettings(){
    const defaults=defaultSettings();
    const parent=el('model-settings');
    for(const [key,spec] of Object.entries(SETTINGS_SPEC)){
      const label=document.createElement('label');
      const span=document.createElement('span');
      span.textContent=settingNames[key] ? translate(locale,settingNames[key]):key.replaceAll('_',' ');
      if(settingNames[key])span.dataset.i18n=settingNames[key];
      const input=document.createElement('input');
      input.id='model-setting-'+key;
      input.dataset.setting=key;
      if(spec[3]==='boolean'){
        input.type='checkbox';
        input.checked=defaults[key];
      }else{
        input.type='number';
        input.min=String(spec[0]);input.max=String(spec[1]);
        input.step=spec[3]==='integer'?'1':'any';
        input.value=String(defaults[key]);
      }
      label.append(span,input);
      parent.append(label);
    }
  }
  function readSettings(){
    const settings={};
    for(const [key,spec] of Object.entries(SETTINGS_SPEC)){
      const input=el('model-setting-'+key);
      settings[key]=spec[3]==='boolean'?input.checked:Number(input.value);
    }
    return validateSettings(settings);
  }
  function displaySettings(values){
    for(const [key,spec] of Object.entries(SETTINGS_SPEC)){
      const input=el('model-setting-'+key);
      if(spec[3]==='boolean')input.checked=values[key];
      else input.value=String(values[key]);
    }
  }
  function selectModelKind(kind){
    el('model-api-controls').hidden=kind!=='api';
    el('model-hf-controls').hidden=kind!=='huggingface';
    el('model-file-controls').hidden=kind!=='file';
    updateDownloadLink();
  }
  function updateDownloadLink(){
    const anchor=el('hf-download');
    const path=el('hf-file').value;
    const selected=catalog?.files.find(file=>file.path===path);
    if(!selected){anchor.hidden=true;anchor.removeAttribute('href');return;}
    anchor.href=selected.download_url;
    anchor.hidden=false;
    // Cross-origin downloads are handled by the browser after an explicit click.
    anchor.title=(selected.size_bytes===null?'Rozmiar nieznany':
      (selected.size_bytes/1024**3).toFixed(2)+' GiB')+' · SHA '+catalog.commit_sha;
  }
  async function searchHf(){
    const repo=el('hf-repo').value.trim();
    catalog=null;updateDownloadLink();
    el('hf-status').textContent='Sprawdzanie publicznego repozytorium Hugging Face…';
    const listing=await listHuggingFaceGguf(repo);
    catalog=listing;
    const chooser=el('hf-file');
    chooser.replaceChildren();
    for(const entry of listing.files){
      const option=document.createElement('option');
      option.value=entry.path;
      option.textContent=entry.path+' ('+(entry.size_bytes===null?'?':(entry.size_bytes/1024**3).toFixed(2)+' GiB')+')';
      chooser.append(option);
    }
    updateDownloadLink();
    el('hf-status').textContent=listing.files.length+' plików GGUF · commit '+listing.commit_sha+
      (listing.gated?' · repozytorium wymaga autoryzacji Hugging Face':'');
    if(!listing.files.length)throw Error('huggingface: no downloadable GGUF files found');
  }
  async function selectLocalFile(event){
    localGguf=null;
    const file=event.target.files?.[0];
    if(!file)return;
    const inspection=await inspectLocalGguf(file);
    // Only a temporary File handle and its bounded header are retained.
    // No model weights, file paths or API credentials are stored.
    localGguf=inspection;
    el('gguf-status').textContent=inspection.filename+' · '+
      (inspection.size_bytes/1024**3).toFixed(2)+' GiB · GGUF v'+inspection.header.version+
      ' · '+inspection.header.tensor_count+' tensorów'+
      (inspection.header.metadata_incomplete?' · metadane częściowe':'');
  }
  function createProfile(){
    const kind=el('model-kind').value;
    const settings=readSettings();
    if(kind==='api'){
      return makeModelProfile({kind:'api',provider:el('model-provider').value,model_id:el('model-id').value.trim()},settings);
    }
    if(kind==='huggingface'){
      const selected=catalog?.files.find(file=>file.path===el('hf-file').value);
      if(!selected||catalog.repo_id!==el('hf-repo').value.trim())throw Error('huggingface: najpierw wyszukaj i wybierz GGUF');
      return makeModelProfile({kind:'huggingface',repo_id:catalog.repo_id,
        commit_sha:catalog.commit_sha,path:selected.path,size_bytes:selected.size_bytes},settings);
    }
    if(kind==='file'){
      if(!localGguf)throw Error('gguf: najpierw wybierz i zweryfikuj lokalny plik');
      return makeModelProfile({kind:'file',filename:localGguf.filename,
        size_bytes:localGguf.size_bytes,gguf_version:localGguf.header.version},settings);
    }
    throw Error('profile: invalid model source');
  }
  function save(){
    const {tenant,projectId}=context();
    activeProfile=saveModelProfile(localStorage,tenant,projectId,createProfile());
    el('model-context').textContent='Projekt '+projectId+' · '+tenant+
      ' · ustawienia zapisane wyłącznie w tej przeglądarce.';
    emit('Profil zapisany. Model nie został uruchomiony; żaden klucz API nie został wyeksportowany.');
  }
  function exportProfile(){
    const {tenant,projectId}=context();
    const profile=loadModelProfile(localStorage,tenant,projectId);
    if(!profile)throw Error('profile: zapisz profil przed eksportem');
    const blob=new Blob([exportModelProfile(profile)],{type:'application/json'});
    const url=URL.createObjectURL(blob);
    const anchor=document.createElement('a');
    anchor.href=url;anchor.download=projectId+'.ocaf-model-profile.json';
    anchor.click();
    setTimeout(()=>URL.revokeObjectURL(url),1000);
    emit('Wyeksportowano wyłącznie ustawienia i identyfikator modelu. Bez danych uwierzytelniających.');
  }
  function fillProfile(profile){
    el('model-kind').value=profile.source.kind;
    selectModelKind(profile.source.kind);
    displaySettings(profile.settings);
    if(profile.source.kind==='api'){
      el('model-provider').value=profile.source.provider;
      el('model-id').value=profile.source.model_id;
    }else if(profile.source.kind==='huggingface'){
      const source=profile.source;
      el('hf-repo').value=source.repo_id;
      const download_url='https://huggingface.co/'+source.repo_id+
        '/resolve/'+source.commit_sha+'/'+source.path.split('/').map(encodeURIComponent).join('/')+'?download=true';
      catalog={repo_id:source.repo_id,commit_sha:source.commit_sha,
        files:[{path:source.path,size_bytes:source.size_bytes,download_url}]};
      const option=document.createElement('option');
      option.value=source.path;option.textContent=source.path;
      el('hf-file').replaceChildren(option);updateDownloadLink();
    }else{
      el('gguf-status').textContent='Zapisano '+profile.source.filename+
        '. Wybierz plik ponownie, jeśli chcesz go użyć w przyszłym runnerze.';
    }
  }
  function refresh(){
    const ctx=getContext();
    const next=ctx?.projectId ? ctx.tenant+':'+ctx.projectId+':'+ctx.workspace : '';
    el('model-save').disabled=!next;
    el('model-export').disabled=!next;
    if(el('runner-start'))el('runner-start').disabled=!next||ctx.workspace!=='local'||!el('runner-model').value;
    if(contextKey===next)return;
    contextKey=next;
    catalog=null;localGguf=null;activeProfile=null;
    el('hf-file').replaceChildren();updateDownloadLink();
    el('hf-status').textContent='';el('gguf-status').textContent='';
    displaySettings(defaultSettings());
    if(!next){el('model-context').textContent='Wybierz projekt, aby przypisać ustawienia.';return;}
    const profile=loadModelProfile(localStorage,ctx.tenant,ctx.projectId);
    activeProfile=profile;
    el('model-context').textContent=profile ?
      'Profil przypisany lokalnie do projektu '+ctx.projectId+' (niezsynchronizowany z chmurą).':
      'Brak profilu modelu dla projektu '+ctx.projectId+'.';
    if(profile)fillProfile(profile);
  }
  function initRunner(){
    const label=el('runner-status');
    const setStatus=text=>{label.textContent=text;};
    const connect=async()=>{
      if(getContext().workspace!=='local')throw Error('runner: przełącz na lokalny projekt');
      if(runnerClient)runnerClient.disconnect();
      runnerClient=null;
      const token=el('runner-token').value.trim();
      let client;
      try{client=new LocalRunnerClient({token,origin:location.origin});}
      finally{el('runner-token').value='';}
      try{
        const [models,status]=await Promise.all([client.models(),client.status()]);
        runnerClient=client;
        showModels(models);
        showStatus(status);
      }catch(error){
        client.disconnect();
        throw error;
      }
    };
    const showModels=models=>{
      const selected=el('runner-model').value;
      el('runner-model').replaceChildren();
      for(const model of models){
        const option=document.createElement('option');
        option.value=model.name;
        option.textContent=model.name+' · '+(model.size_bytes/1024**3).toFixed(2)+' GiB · GGUF v'+model.version;
        el('runner-model').append(option);
      }
      if(models.some(m=>m.name===selected))el('runner-model').value=selected;
      el('runner-start').disabled=!models.length||getContext().workspace!=='local';
    };
    const showStatus=status=>{
      setStatus('Stan lokalnego llama.cpp: '+status.state+(status.model?' · '+status.model:''));
    };
    const refresh=async()=>{
      if(!runnerClient)throw Error('runner: najpierw połącz z lokalnym serwerem');
      const [models,status]=await Promise.all([runnerClient.models(),runnerClient.status()]);
      showModels(models);showStatus(status);
    };
    el('runner-connect').addEventListener('click',safe(connect));
    el('runner-disconnect').addEventListener('click',safe(()=>{
      runnerClient?.disconnect();runnerClient=null;
      el('runner-token').value='';
      el('runner-model').replaceChildren();
      el('runner-start').disabled=true;
      setStatus('Rozłączono. Proces modelu pozostaje pod kontrolą lokalnego serwera.');
    }));
    el('runner-refresh').addEventListener('click',safe(refresh));
    el('runner-start').addEventListener('click',safe(async()=>{
      if(!runnerClient)throw Error('runner: brak sesji');
      if(getContext().workspace!=='local')throw Error('runner: tylko tryb Local');
      const model=el('runner-model').value;
      const response=await runnerClient.start(model,readSettings());
      showStatus(response);
      emit('Uruchamianie GGUF przez lokalny llama-server. Odśwież stan przed inferencją.');
    }));
    el('runner-stop').addEventListener('click',safe(async()=>{
      if(!runnerClient)throw Error('runner: brak sesji');
      showStatus(await runnerClient.stop());
    }));
    el('runner-send').addEventListener('click',safe(async()=>{
      if(!runnerClient)throw Error('runner: brak sesji');
      if(getContext().workspace!=='local')throw Error('runner: tylko tryb Local');
      el('runner-answer').textContent='Inferencja lokalna trwa…';
      try{
        const answer=await runnerClient.chat(el('runner-prompt').value);
        el('runner-answer').textContent=answer.text;
        showStatus({state:'ready',model:answer.model});
      }catch(error){
        el('runner-answer').textContent='Brak wyniku.';
        throw error;
      }
    }));
    window.addEventListener('pagehide',()=>runnerClient?.disconnect(),{once:true});
    setStatus('Połącz z serwerem uruchomionym na tym samym urządzeniu.');
  }
  function init(){
    initProviders();
    initRunner();
    initSettings();
    updateLanguage();
    el('ui-language').addEventListener('change',safe(event=>setLocale(event.target.value)));
    el('locale-catalog').addEventListener('change',safe(importLocale));
    el('model-kind').addEventListener('change',()=>selectModelKind(el('model-kind').value));
    el('hf-search').addEventListener('click',safe(searchHf));
    el('hf-file').addEventListener('change',updateDownloadLink);
    el('hf-repo').addEventListener('input',()=>{
      if(catalog?.repo_id!==el('hf-repo').value.trim()){catalog=null;updateDownloadLink();}
    });
    el('gguf-file').addEventListener('change',safe(selectLocalFile));
    el('model-save').addEventListener('click',safe(save));
    el('model-export').addEventListener('click',safe(exportProfile));
    selectModelKind('api');
    refresh();
  }
  init();
  return refresh;
}
