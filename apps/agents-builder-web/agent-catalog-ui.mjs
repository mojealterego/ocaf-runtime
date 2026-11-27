/** Read-only public fleet browser. This never grants agent execution or licenses. */
import {fetchPublicCatalog,searchPublicCatalogPage} from './agent-catalog.mjs';

const LABELS = Object.freeze({
  DRAFT:'Projekt / DRAFT', REFERENCE_ONLY:'Rój referencyjny',
  NOT_DEPLOYED:'Niewdrożony', NOT_BUILT:'Brak gotowego artefaktu',
  NOT_VERIFIED:'Dokumentacja niezweryfikowana',
  NOT_ISSUED:'Certyfikat niewydany', NOT_PRICED:'Brak potwierdzonej ceny'
});

export async function initPublicCatalog({document,fetchImpl}={}) {
  if (!document || typeof document.getElementById !== 'function') throw Error('catalog: invalid DOM');
  const el=id=>{
    const node=document.getElementById(id);
    if(!node)throw Error('catalog: missing DOM node '+id);
    return node;
  };
  const input=el('catalog-query'), list=el('catalog-list');
  const status=el('catalog-status'), detail=el('catalog-detail');
  const prev=el('catalog-prev'), next=el('catalog-next');
  const PAGE_SIZE=30;
  let catalog=null, offset=0;
  prev.disabled=next.disabled=true;
  const clearDetail=()=>detail.replaceChildren();
  function show(selection) {
    clearDetail();
    const id=typeof selection==='string'?selection:selection?.id;
    const entry=catalog&&[...catalog.agents,...catalog.swarms].find(item=>item.id===id);
    if(!entry)throw Error('catalog: unknown dossier');
    const isAgent=Object.hasOwn(entry,'group_id');
    const groupId=isAgent?`G${Number(entry.group_id.slice(1))}`:entry.id;
    const swarm=catalog.swarms.find(item=>item.id===groupId);
    const members=catalog.agents.filter(item=>`G${Number(item.group_id.slice(1))}`===groupId);
    const lines=[
      ['Identyfikator',entry.id],
      ['Nazwa / specjalizacja',entry.display_name||entry.specialization],
      ['Specjalizacja',entry.specialization],
      ['Rój',entry.group_id||entry.id],
      ['Zakres roju',swarm.scope],
      ['Członkowie roju (rejestr źródłowy)',members.map(item=>item.id).join(', ')||'Brak wpisów'],
      ['Flota',catalog.fleet_id],
      ['Wersja schematu katalogu',String(catalog.schema_version)],
      ['Git blob rejestru agentów (deklarowany)',catalog.source_agent_registry_git_blob],
      ['Git blob rejestru rojów (deklarowany)',catalog.source_swarm_registry_git_blob],
      ['Autentyczność źródła','Niezweryfikowane kryptograficznie'],
      ['Cykl życia',LABELS[entry.lifecycle]],
      ['Wdrożenie',LABELS[entry.deployment_status]],
      ...(isAgent?[['Artefakt',LABELS[entry.artifact_status]]]:[]),
      ['Dokumentacja',LABELS[entry.documentation_status]],
      ['Certyfikacja',LABELS[entry.certification_status]],
      ['Wycena',LABELS[entry.price_status]],
      ['Możliwość uruchomienia','ZABLOKOWANA — brak zweryfikowanego wykonawcy i dopuszczenia']
    ];
    const dl=document.createElement('dl');
    for(const [key,value] of lines){
      const dt=document.createElement('dt'),dd=document.createElement('dd');
      dt.textContent=key;dd.textContent=value;
      dl.append(dt,dd);
    }
    detail.append(dl);
  }
  function render() {
    list.replaceChildren();clearDetail();
    prev.disabled=next.disabled=true;
    if(!catalog)return;
    let rows,total;
    try { ({rows,total}=searchPublicCatalogPage(catalog,input.value,PAGE_SIZE,offset)); }
    catch(error){status.textContent=error.message;return;}
    for(const entry of rows){
      const item=document.createElement('li'),button=document.createElement('button');
      button.type='button';button.className='secondary';
      button.textContent=(entry.display_name||entry.id)+' · '+entry.specialization+' · '+
        (entry.lifecycle==='REFERENCE_ONLY'?'RÓJ REFERENCYJNY':'DRAFT');
      button.addEventListener('click',()=>show(entry.id));
      item.append(button);list.append(item);
    }
    prev.disabled=offset===0;
    next.disabled=offset+rows.length>=total;
    status.textContent=`Rejestr źródłowy: ${catalog.agents.length} agentów DRAFT, `+
      `${catalog.swarms.length} rojów referencyjnych, `+
      `${catalog.active_runtime_agent_count} aktywnych. Pokazano ${total?offset+1:0}–${offset+rows.length} z ${total} pozycji. `+
      'Brak certyfikacji, wycen i autoryzacji uruchomienia.';
  }
  input.addEventListener('input',()=>{offset=0;render();});
  prev.addEventListener('click',()=>{if(!prev.disabled){offset-=PAGE_SIZE;render();}});
  next.addEventListener('click',()=>{if(!next.disabled){offset+=PAGE_SIZE;render();}});
  try {
    status.textContent='Wczytywanie rejestru źródłowego…';
    catalog=await fetchPublicCatalog({fetchImpl});
    render();
    return {render,select:show};
  } catch(error) {
    catalog=null;list.replaceChildren();clearDetail();
    prev.disabled=next.disabled=true;
    status.textContent='Katalog niedostępny: brak zweryfikowanych danych źródłowych.';
    throw error;
  }
}
