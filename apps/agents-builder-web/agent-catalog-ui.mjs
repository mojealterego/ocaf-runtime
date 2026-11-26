/** Read-only public fleet browser. This never grants agent execution or licenses. */
import {fetchPublicCatalog,searchPublicCatalog} from './agent-catalog.mjs';

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
  let catalog=null;
  const clearDetail=()=>detail.replaceChildren();
  function show(entry) {
    clearDetail();
    const lines=[
      ['Identyfikator',entry.id],
      ['Nazwa / specjalizacja',entry.display_name||entry.specialization],
      ['Specjalizacja',entry.specialization],
      ['Rój',entry.group_id||entry.id],
      ['Cykl życia',LABELS[entry.lifecycle]],
      ['Wdrożenie',LABELS[entry.deployment_status]],
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
    if(!catalog)return;
    let rows;
    try { rows=searchPublicCatalog(catalog,input.value,30); }
    catch(error){status.textContent=error.message;return;}
    for(const entry of rows){
      const item=document.createElement('li'),button=document.createElement('button');
      button.type='button';button.className='secondary';
      button.textContent=(entry.display_name||entry.id)+' · '+entry.specialization+' · '+
        (entry.lifecycle==='REFERENCE_ONLY'?'RÓJ REFERENCYJNY':'DRAFT');
      button.addEventListener('click',()=>show(entry));
      item.append(button);list.append(item);
    }
    status.textContent=`Rejestr źródłowy: ${catalog.agents.length} agentów DRAFT, `+
      `${catalog.swarms.length} rojów referencyjnych, `+
      `${catalog.active_runtime_agent_count} aktywnych. Pokazano ${rows.length} pozycji. `+
      'Brak certyfikacji, wycen i autoryzacji uruchomienia.';
  }
  input.addEventListener('input',render);
  try {
    status.textContent='Wczytywanie rejestru źródłowego…';
    catalog=await fetchPublicCatalog({fetchImpl});
    render();
    return {render,select:show};
  } catch(error) {
    catalog=null;list.replaceChildren();clearDetail();
    status.textContent='Katalog niedostępny: brak zweryfikowanych danych źródłowych.';
    throw error;
  }
}
