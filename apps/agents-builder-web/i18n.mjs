/**
 * OCAF Atelier UI translations. Polish is the offline fallback.
 * UI strings are always assigned via textContent, never as HTML.
 * Custom catalogs are untrusted text and are not automatically persisted.
 */
export const LOCALES = Object.freeze([
  ['pl-PL','Polski'], ['en-US','English'], ['cs-CZ','Čeština'],
  ['sk-SK','Slovenčina'], ['de-DE','Deutsch'], ['uk-UA','Українська'],
  ['ar-SA','العربية']
]);
const pl = {
  brand:'OCAF Agents Builder',subtitle:'Code / NoCode · Lokalnie / Chmura',
  title:'Projektuj agentów.',description:'Jeden typowany projekt. Dwa edytory. Dwa środowiska.',
  local:'Lokalnie',cloud:'Chmura',language:'Język',
  project:'Identyfikator projektu',new:'Nowy',open:'Otwórz',
  import:'Importuj JSON',export:'Eksportuj JSON',history:'Historia',restore:'Przywróć',
  code:'Code · DSL',nocode:'NoCode · Graf',validate:'Waliduj / Symuluj',
  palette:'Paleta węzłów',palette_help:'Wstaw węzeł albo przeciągnij go na płótno.',
  node_id:'Identyfikator węzła',transform:'Transformacja',tool:'Narzędzie',
  tool_grant:'Uprawnienie narzędzia',tool_cost:'Maksymalny koszt narzędzia',
  budget:'Budżet projektu',budget_update:'Zmień budżet',grants:'Zadeklarowane uprawnienia',
  canvas:'Graf przepływu',canvas_help:'Wybierz węzeł lub połączenie.',
  edges:'Typowane połączenia',remove_node:'Usuń wybrany węzeł',
  selected_tool:'Wybrane narzędzie',tool_name:'Nazwa narzędzia',tool_update:'Zmień narzędzie',
  code_title:'Edytor JSON DSL',apply_code:'Waliduj i zastosuj Code',
  preview:'Walidacja i podgląd',boundary:'Granica wykonania',
  connector:'Połączenie chmurowe',cloud_help:'Wymaga uwierzytelnionego gateway HTTPS i tokenu OAuth. Bez automatycznej publikacji.',
  tenant:'Tenant',oauth:'Token OAuth',connect:'Połącz',disconnect:'Rozłącz',
  open_cloud:'Otwórz z chmury',new_cloud:'Nowy w chmurze',save_cloud:'Zapisz w chmurze',
  integrations:'Modele i dostawcy API',api_keys:'Klucze API · tylko bieżąca sesja',
  provider:'Dostawca',secret:'Klucz API',add_key:'Dodaj / zamień klucz',
  remove_key:'Usuń klucz',api_notice:'Klucze pozostają tylko w pamięci tej karty. Nie trafiają do projektów ani eksportu.',
  model_source:'Źródło modelu',model_type:'Typ profilu',api_model:'Model API',
  model_id:'Identyfikator modelu',gguf:'GGUF · Hugging Face',
  hf_repo:'Repozytorium Hugging Face',hf_search:'Wyszukaj pliki GGUF',
  hf_file:'Plik GGUF',hf_download:'Pobierz GGUF z Hugging Face',
  local_gguf:'Wczytaj lokalny plik GGUF',runtime:'Ustawienia inferencji',
  save_profile:'Zapisz profil modelu',export_profile:'Eksportuj profil JSON',
  model_notice:'Profil konfiguracji nie uruchamia inferencji. Wymagany zgodny silnik lokalny lub chmurowy.',
  profile_status:'Status profilu',ctx:'Kontekst (tokeny)',threads:'Wątki CPU',
  batch:'Rozmiar batch',gpu_layers:'Warstwy GPU',temp:'Temperatura',
  top_p:'Top P',top_k:'Top K',min_p:'Min P',repeat:'Kara powtórzeń',
  seed:'Seed (-1 = losowy)',max_tokens:'Maks. tokenów odpowiedzi',
  flash:'Flash Attention',mmap:'Mapowanie pliku (mmap)',
  runner_title:'Lokalny runner GGUF · llama.cpp',runner_desc:'Uruchom na tym urządzeniu lokalny serwer Python i llama-server. Modele tylko z dozwolonego katalogu. Żadnego wykonywania narzędzi agentów.',runner_token:'Token lokalnej sesji',runner_connect:'Połącz runner',runner_disconnect:'Rozłącz runner',runner_refresh:'Odśwież modele / stan',runner_model:'Lokalny plik GGUF',runner_start:'Załaduj i uruchom model',runner_stop:'Zatrzymaj model',runner_prompt:'Wiadomość dla modelu',runner_send:'Generuj lokalnie',
  locale_import:'Wczytaj katalog tłumaczeń JSON'
};
const en = {
  title:'Build agents.',description:'One typed project. Two editors. Two environments.',
  subtitle:'Code / NoCode · Local / Cloud',local:'Local',cloud:'Cloud',language:'Language',
  project:'Project ID',new:'New',open:'Open',import:'Import JSON',
  export:'Export JSON',history:'History',restore:'Restore',
  code:'Code · DSL',nocode:'NoCode · Graph',validate:'Validate / Dry run',
  palette:'Node palette',palette_help:'Insert or drag a node onto the canvas.',
  node_id:'Node ID',transform:'Transform',tool:'Tool',tool_grant:'Tool permission',
  tool_cost:'Maximum tool cost',budget:'Project budget',budget_update:'Update budget',
  grants:'Declared grants',canvas:'Workflow graph',
  canvas_help:'Select a node or connection.',edges:'Typed connections',
  remove_node:'Remove selected node',selected_tool:'Selected tool',
  tool_name:'Tool name',tool_update:'Update tool',code_title:'JSON DSL editor',
  apply_code:'Validate and apply Code',preview:'Validation and preview',
  boundary:'Execution boundary',connector:'Cloud connection',
  cloud_help:'Requires an authenticated HTTPS gateway and OAuth token. No automatic deployment.',
  tenant:'Tenant',oauth:'OAuth token',connect:'Connect',disconnect:'Disconnect',
  open_cloud:'Open from cloud',new_cloud:'New in cloud',save_cloud:'Save to cloud',
  integrations:'Models and API providers',api_keys:'API keys · current session only',
  provider:'Provider',secret:'API key',add_key:'Add / replace key',
  remove_key:'Remove key',api_notice:'Keys remain only in memory of this tab. They never enter projects or exports.',
  model_source:'Model source',model_type:'Profile type',api_model:'API model',
  model_id:'Model identifier',gguf:'GGUF · Hugging Face',
  hf_repo:'Hugging Face repository',hf_search:'Find GGUF files',
  hf_file:'GGUF file',hf_download:'Download GGUF from Hugging Face',
  local_gguf:'Select local GGUF file',runtime:'Inference settings',
  save_profile:'Save model profile',export_profile:'Export profile JSON',
  model_notice:'A configuration profile does not execute inference. A compatible local or cloud runner is required.',
  profile_status:'Profile status',ctx:'Context (tokens)',threads:'CPU threads',
  batch:'Batch size',gpu_layers:'GPU layers',temp:'Temperature',
  top_p:'Top P',top_k:'Top K',min_p:'Min P',repeat:'Repeat penalty',
  seed:'Seed (-1 = random)',max_tokens:'Max output tokens',
  runner_title:'Local GGUF runner · llama.cpp',runner_desc:'Run the local Python server and llama-server on this device. Models only from the approved directory. No agent tools are executed.',runner_token:'Local session token',runner_connect:'Connect runner',runner_disconnect:'Disconnect runner',runner_refresh:'Refresh models / status',runner_model:'Local GGUF file',runner_start:'Load and start model',runner_stop:'Stop model',runner_prompt:'Message for model',runner_send:'Generate locally',
  flash:'Flash Attention',mmap:'Memory map (mmap)',locale_import:'Load translation catalog JSON'
};
const cs={title:'Navrhujte agenty.',description:'Jeden typovaný projekt. Dva editory. Dvě prostředí.',local:'Lokálně',cloud:'Cloud',language:'Jazyk',
 integrations:'Modely a poskytovatelé API',api_keys:'Klíče API · pouze aktuální relace',provider:'Poskytovatel',secret:'Klíč API',add_key:'Přidat / nahradit klíč',remove_key:'Smazat klíč',
 api_notice:'Klíče zůstávají pouze v paměti této karty.',hf_repo:'Repozitář Hugging Face',hf_search:'Vyhledat soubory GGUF',hf_file:'Soubor GGUF',hf_download:'Stáhnout GGUF',local_gguf:'Vybrat místní soubor GGUF',
 runner_title:'Lokální GGUF runner · llama.cpp',runner_desc:'Spusťte místní Python server a llama-server na tomto zařízení. Modely jen z povolené složky. Bez spouštění nástrojů agentů.',runner_token:'Token místní relace',runner_connect:'Připojit',runner_disconnect:'Odpojit',runner_refresh:'Obnovit modely / stav',runner_model:'Místní GGUF soubor',runner_start:'Načíst a spustit model',runner_stop:'Zastavit model',runner_prompt:'Zpráva pro model',runner_send:'Generovat lokálně',
  save_profile:'Uložit profil modelu',model_notice:'Profil nespouští inferenci. Je potřeba kompatibilní runtime.',validate:'Ověřit / simulovat',import:'Importovat JSON',export:'Exportovat JSON',new:'Nový',open:'Otevřít',connect:'Připojit',disconnect:'Odpojit'};
const sk={title:'Navrhujte agentov.',description:'Jeden typovaný projekt. Dva editory. Dve prostredia.',local:'Lokálne',cloud:'Cloud',language:'Jazyk',
 integrations:'Modely a poskytovatelia API',api_keys:'Kľúče API · iba aktuálna relácia',provider:'Poskytovateľ',secret:'Kľúč API',add_key:'Pridať / nahradiť kľúč',remove_key:'Odstrániť kľúč',
 api_notice:'Kľúče zostávajú iba v pamäti tejto karty.',hf_repo:'Repozitár Hugging Face',hf_search:'Vyhľadať súbory GGUF',hf_file:'Súbor GGUF',hf_download:'Stiahnuť GGUF',local_gguf:'Vybrať miestny súbor GGUF',
 runner_title:'Lokálny GGUF runner · llama.cpp',runner_desc:'Spustite lokálny Python server a llama-server na tomto zariadení. Modely len z povoleného priečinka. Bez spustenia agentných nástrojov.',runner_token:'Token lokálnej relácie',runner_connect:'Pripojiť',runner_disconnect:'Odpojiť',runner_refresh:'Obnoviť modely / stav',runner_model:'Lokálny GGUF súbor',runner_start:'Načítať a spustiť model',runner_stop:'Zastaviť model',runner_prompt:'Správa pre model',runner_send:'Generovať lokálne',
  save_profile:'Uložiť profil modelu',model_notice:'Profil nespúšťa inferenciu. Vyžaduje kompatibilný runtime.',validate:'Overiť / simulovať',import:'Importovať JSON',export:'Exportovať JSON',new:'Nový',open:'Otvoriť',connect:'Pripojiť',disconnect:'Odpojiť'};
const de={title:'Agenten entwickeln.',description:'Ein typisiertes Projekt. Zwei Editoren. Zwei Umgebungen.',local:'Lokal',cloud:'Cloud',language:'Sprache',
 integrations:'Modelle und API-Anbieter',api_keys:'API-Schlüssel · nur aktuelle Sitzung',provider:'Anbieter',secret:'API-Schlüssel',add_key:'Schlüssel hinzufügen / ersetzen',remove_key:'Schlüssel entfernen',
 api_notice:'Schlüssel bleiben nur im Speicher dieses Tabs.',hf_repo:'Hugging Face Repository',hf_search:'GGUF-Dateien suchen',hf_file:'GGUF-Datei',hf_download:'GGUF herunterladen',local_gguf:'Lokale GGUF-Datei auswählen',
 runner_title:'Lokaler GGUF-Runner · llama.cpp',runner_desc:'Starten Sie den lokalen Python-Server und llama-server auf diesem Gerät. Nur Modelle aus dem freigegebenen Verzeichnis. Keine Agenten-Tools.',runner_token:'Lokales Sitzungstoken',runner_connect:'Runner verbinden',runner_disconnect:'Verbindung trennen',runner_refresh:'Modelle / Status aktualisieren',runner_model:'Lokale GGUF-Datei',runner_start:'Modell laden und starten',runner_stop:'Modell stoppen',runner_prompt:'Nachricht an das Modell',runner_send:'Lokal generieren',
  save_profile:'Modellprofil speichern',model_notice:'Das Profil führt keine Inferenz aus. Eine kompatible Laufzeit ist erforderlich.',validate:'Prüfen / Simulieren',import:'JSON importieren',export:'JSON exportieren',new:'Neu',open:'Öffnen',connect:'Verbinden',disconnect:'Trennen'};
const uk={title:'Створюйте агентів.',description:'Один типізований проєкт. Два редактори. Два середовища.',local:'Локально',cloud:'Хмара',language:'Мова',
 integrations:'Моделі й постачальники API',api_keys:'Ключі API · лише поточний сеанс',provider:'Постачальник',secret:'Ключ API',add_key:'Додати / замінити ключ',remove_key:'Видалити ключ',
 api_notice:'Ключі зберігаються лише в пам’яті цієї вкладки.',hf_repo:'Репозиторій Hugging Face',hf_search:'Знайти файли GGUF',hf_file:'Файл GGUF',hf_download:'Завантажити GGUF',local_gguf:'Вибрати локальний файл GGUF',
 runner_title:'Локальний GGUF-рушій · llama.cpp',runner_desc:'Запустіть локальний Python-сервер і llama-server на цьому пристрої. Моделі лише з дозволеного каталогу. Без інструментів агентів.',runner_token:'Токен локального сеансу',runner_connect:'Підключити рушій',runner_disconnect:'Відключити',runner_refresh:'Оновити моделі / стан',runner_model:'Локальний файл GGUF',runner_start:'Завантажити й запустити',runner_stop:'Зупинити модель',runner_prompt:'Повідомлення для моделі',runner_send:'Генерувати локально',
  save_profile:'Зберегти профіль моделі',model_notice:'Профіль не запускає інференс. Потрібен сумісний рушій.',validate:'Перевірити / симулювати',import:'Імпорт JSON',export:'Експорт JSON',new:'Новий',open:'Відкрити',connect:'Підключити',disconnect:'Відключити'};
const ar={title:'أنشئ الوكلاء.',description:'مشروع واحد بنوع محدد. محرران. بيئتان.',local:'محلي',cloud:'سحابة',language:'اللغة',
 integrations:'النماذج ومزودو واجهات البرمجة',api_keys:'مفاتيح API · الجلسة الحالية فقط',provider:'المزود',secret:'مفتاح API',add_key:'إضافة / استبدال المفتاح',remove_key:'حذف المفتاح',
 api_notice:'تبقى المفاتيح في ذاكرة علامة التبويب فقط.',hf_repo:'مستودع Hugging Face',hf_search:'البحث عن ملفات GGUF',hf_file:'ملف GGUF',hf_download:'تنزيل GGUF',local_gguf:'اختيار ملف GGUF محلي',
 runner_title:'مشغّل GGUF المحلي · llama.cpp',runner_desc:'شغّل خادم Python المحلي و llama-server على هذا الجهاز. النماذج من المجلد المسموح فقط. دون تنفيذ أدوات الوكلاء.',runner_token:'رمز الجلسة المحلية',runner_connect:'اتصال المشغّل',runner_disconnect:'قطع الاتصال',runner_refresh:'تحديث النماذج / الحالة',runner_model:'ملف GGUF محلي',runner_start:'تحميل النموذج وتشغيله',runner_stop:'إيقاف النموذج',runner_prompt:'رسالة للنموذج',runner_send:'توليد محلي',
  save_profile:'حفظ إعدادات النموذج',model_notice:'الإعداد لا يشغل الاستدلال. يلزم محرك متوافق.',validate:'تحقق / محاكاة',import:'استيراد JSON',export:'تصدير JSON',new:'جديد',open:'فتح',connect:'اتصال',disconnect:'قطع الاتصال'};
const DICTS={'pl-PL':pl,'en-US':en,'cs-CZ':cs,'sk-SK':sk,'de-DE':de,'uk-UA':uk,'ar-SA':ar};
const extra=new Map();
const validTag=value=>typeof value==='string' && value.length<=35 && /^[a-zA-Z]{2,8}(?:-[a-zA-Z0-9]{2,8})*$/.test(value);
export function registerCatalog(locale,catalog){
  if(!validTag(locale)||!catalog||typeof catalog!=='object'||Array.isArray(catalog)||Object.keys(catalog).length>160)throw Error('locale: invalid catalog');
  const messages=Object.create(null);
  for(const [key,value] of Object.entries(catalog)){
    if(!Object.hasOwn(pl,key)||typeof value!=='string'||value.length>1024)throw Error('locale: unknown key or invalid message');
    messages[key]=value;
  }
  extra.set(locale,messages);
}
export function supportedLocales(){
  return [...LOCALES,...[...extra.keys()].filter(x=>!LOCALES.some(([id])=>id===x)).map(id=>[id,id])];
}
export function translate(locale,key,vars={}){
  if(!Object.hasOwn(pl,key))throw Error('locale: invalid UI message key');
  const dict=extra.get(locale)??DICTS[locale]??pl;
  let message=dict[key]??pl[key];
  for(const [name,value] of Object.entries(vars)){
    if(/^[a-z][a-z0-9_]{0,30}$/i.test(name))message=message.replaceAll('{'+name+'}',String(value).slice(0,128));
  }
  return message;
}
export function plural(locale,count,forms){
  if(!Number.isFinite(count)||!forms||typeof forms!=='object')throw Error('locale: invalid plural');
  const kind=new Intl.PluralRules(validTag(locale)?locale:'pl-PL').select(count);
  return forms[kind]??forms.other??'';
}
export function applyLanguage(document,locale){
  if(!validTag(locale))throw Error('locale: unsupported tag');
  const rtl=/^(ar|he|fa|ur)(-|$)/i.test(locale);
  document.documentElement.lang=locale;
  document.documentElement.dir=rtl?'rtl':'ltr';
  for(const node of document.querySelectorAll('[data-i18n]')){
    node.textContent=translate(locale,node.dataset.i18n);
  }
  for(const node of document.querySelectorAll('[data-i18n-placeholder]')){
    node.placeholder=translate(locale,node.dataset.i18nPlaceholder);
  }
  return rtl?'rtl':'ltr';
}
