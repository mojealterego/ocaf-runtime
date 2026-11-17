/* Cloud workspace: separate drafts, single active network operation, memory-only token. */
(function(root){
  'use strict';
  var get=function(id){return document.getElementById(id);};
  var picker=get('canonical-workspace');if(!picker)return;
  var source=get('canonical-source'),tenantField=get('canonical-tenant'),pid=get('canonical-id'),transfer=get('canonical-import'),token=get('cloud-token');
  var mode='local',busy=false,snapshot=null,session=null,expected=null,selected=null;
  function say(message){get('cloud-status').textContent=message;}
  function clearSession(){if(session)session.client.disconnect();session=null;expected=null;selected=null;token.value='';}
  function clearView(){source.value='';transfer.value='';get('canonical-result').textContent='';get('canonical-graph-preview').textContent='';}
  function lock(value){
    busy=value;source.readOnly=value;pid.readOnly=value;tenantField.readOnly=value||mode==='cloud';picker.disabled=value;
    ['cloud-connect','cloud-open','cloud-new','cloud-save','cloud-preview','cloud-disconnect'].forEach(function(id){get(id).disabled=value||mode!=='cloud';});
  }
  function discard(){return !source.value||root.confirm('Porzucić bieżący szkic Cloud?');}
  picker.onchange=function(){
    var next=picker.value;
    if(busy||(next!=='local'&&next!=='cloud')){picker.value=mode;return;}if(next===mode)return;
    if(next==='cloud'){
      if(!root.location||root.location.protocol!=='https:'||typeof root.fetch!=='function'){picker.value=mode;say('Cloud niedostępny: wymagany wspólny gateway HTTPS.');return;}
      snapshot={source:source.value,tenant:tenantField.value,id:pid.value,transfer:transfer.value};
      clearView();tenantField.value='';pid.value='';mode=next;say('Cloud: podaj tenant i token OAuth. Bez automatycznej synchronizacji.');
    }else{
      if(!discard()){picker.value=mode;return;}clearSession();clearView();
      source.value=snapshot.source;tenantField.value=snapshot.tenant;pid.value=snapshot.id;transfer.value=snapshot.transfer;
      snapshot=null;mode=next;say('Przywrócono lokalny szkic.');
    }
    get('cloud-panel').hidden=mode!=='cloud';lock(false);
  };
  get('cloud-connect').onclick=function(){
    if(busy||mode!=='cloud')return;var credential=token.value;token.value='';
    try{
      var tenant=OCAFKaiosCloud.identifier(get('cloud-tenant').value);
      var client=OCAFKaiosCloud.createClient(root,tenant,credential,OCAFCanonicalGraph);
      if(session&&!discard()){client.disconnect();return;}
      clearSession();clearView();session={client:client,tenant:tenant};tenantField.value=tenant;pid.value='';
      say('Sesja w pamięci; otwórz projekt, aby sprawdzić dostęp na serwerze.');
    }catch(e){say('Błąd konfiguracji sesji Cloud.');}
  };
  function requireSession(){if(mode!=='cloud'||!session||tenantField.value!==session.tenant)throw Error('Połącz sesję Cloud');return session;}
  function current(active){
    var data=OCAFCanonicalGraph.parseUniqueJson(source.value,131072);
    if(data.project_id!==pid.value||data.project_id!==selected)throw Error('Niezgodny projekt Cloud; otwórz lub utwórz projekt');
    OCAFCanonicalGraph.compile(data,active.tenant);return data;
  }
  function show(data,head){
    var plan=OCAFCanonicalGraph.compile(data,session.tenant);
    source.value=JSON.stringify(data,null,2);transfer.value='';pid.value=data.project_id;selected=data.project_id;expected=head;
    get('canonical-result').textContent=JSON.stringify(plan,null,2);
    get('canonical-graph-preview').textContent=data.edges.map(function(e){return e.from_node+'.'+e.from_port+' → '+e.to_node+'.'+e.to_port;}).join(' | ');
  }
  function fresh(tenant,id){return {schema_version:1,tenant_id:tenant,project_id:id,revision:1,max_cost_minor:0,grants:[],
    nodes:[{id:'input',kind:'input',inputs:[],outputs:[{name:'out',kind:'text'}],tool_name:null,max_cost_minor:0},{id:'output',kind:'output',inputs:[{name:'in',kind:'text'}],outputs:[],tool_name:null,max_cost_minor:0}],
    edges:[{from_node:'input',from_port:'out',to_node:'output',to_port:'in'}]};}
  function run(work){
    if(busy||mode!=='cloud')return Promise.resolve();
    lock(true);return Promise.resolve().then(function(){return work(requireSession());}).then(function(){lock(false);},function(e){say('Błąd Cloud: '+(e&&e.message?e.message:'operacja nieudana'));lock(false);});
  }
  get('cloud-disconnect').onclick=function(){if(busy||mode!=='cloud'||!discard())return;clearSession();clearView();tenantField.value='';pid.value='';say('Rozłączono sesję.');};
  get('cloud-open').onclick=function(){return run(function(s){var id=OCAFKaiosCloud.identifier(pid.value);if(!discard())return;return s.client.load(id).then(function(v){show(v.project,v.head);say('Otworzono projekt Cloud r'+v.head.revision+'.');});});};
  get('cloud-new').onclick=function(){return run(function(s){var id=OCAFKaiosCloud.identifier(pid.value);if(!discard())return;return s.client.load(id).then(function(){throw Error('Projekt istnieje — użyj Otwórz.');},function(e){if(e.status!==404)throw e;show(fresh(s.tenant,id),null);say('Nowy szkic Cloud; nie został zapisany.');});});};
  get('cloud-save').onclick=function(){return run(function(s){var data=current(s),baseline=expected;data.revision=baseline===null?1:baseline.revision+1;
    return s.client.save(data,baseline).then(function(h){show(data,h);say('Zapisano Cloud CAS r'+h.revision+'.');},function(e){selected=null;expected=null;throw e;});});};
  get('cloud-preview').onclick=function(){return run(function(s){return s.client.preview(current(s)).then(function(plan){get('canonical-result').textContent=JSON.stringify(plan,null,2);say('Serwer zwrócił zgodny statyczny plan. Nie wykonano narzędzi.');});});};
  root.OCAFKaiosCloudUI={isBusy:function(){return busy;},isLocal:function(){return mode==='local';}};
  picker.value='local';get('cloud-panel').hidden=true;lock(false);
}(this));
