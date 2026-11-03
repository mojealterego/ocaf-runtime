/**
 * OCAF Agents Builder model profile and GGUF inspection.
 * Configuration only: does not load model weights into an inference engine.
 * Download links are always pinned to a verified Hugging Face commit.
 */
const REPO=/^[A-Za-z0-9][A-Za-z0-9_.-]{0,95}\/[A-Za-z0-9][A-Za-z0-9_.-]{0,95}$/;
const SHA=/^[0-9a-f]{40}$/i;
const ID=/^[A-Za-z0-9_.-]{1,64}$/;
const MAX_JSON=128*1024;
const MAX_GGUF_HEADER=4*1024*1024;
const MAX_HF_RESPONSE=4*1024*1024;
const SETTINGS=Object.freeze({
  context_size:[256,131072,4096,'integer'],
  threads:[1,128,4,'integer'],
  batch_size:[1,8192,256,'integer'],
  gpu_layers:[0,2048,0,'integer'],
  temperature:[0,2,0.7,'number'],
  top_p:[0,1,0.95,'number'],
  top_k:[1,512,40,'integer'],
  min_p:[0,1,0.05,'number'],
  repeat_penalty:[0.5,3,1.1,'number'],
  presence_penalty:[-2,2,0,'number'],
  frequency_penalty:[-2,2,0,'number'],
  seed:[-1,2147483647,-1,'integer'],
  max_output_tokens:[1,32768,1024,'integer'],
  flash_attention:[false,true,false,'boolean'],
  use_mmap:[false,true,true,'boolean']
});
export const SETTINGS_SPEC=SETTINGS;
export function defaultSettings(){
  return Object.fromEntries(Object.entries(SETTINGS).map(([key,spec])=>[key,spec[2]]));
}
const plain=value=>value!==null&&typeof value==='object'&&!Array.isArray(value);
const fields=(obj,expected)=>{
  if(!plain(obj)||Object.keys(obj).some(k=>!expected.includes(k)))throw Error('profile: unsupported field');
  if(expected.some(k=>!Object.hasOwn(obj,k)))throw Error('profile: missing field');
};
const id=value=>typeof value==='string' && ID.test(value) && value!=='.' && value!=='..';
export function validateSettings(value){
  fields(value,Object.keys(SETTINGS));
  for(const [key,spec] of Object.entries(SETTINGS)){
    const n=value[key];
    if(spec[3]==='boolean'){
      if(typeof n!=='boolean')throw Error('settings: invalid '+key);
    } else if(typeof n!=='number'||!Number.isFinite(n)||n<spec[0]||n>spec[1]||
              (spec[3]==='integer'&&!Number.isSafeInteger(n))){
      throw Error('settings: invalid '+key);
    }
  }
  if(value.max_output_tokens>value.context_size)throw Error('settings: max output exceeds context');
  return structuredClone(value);
}
export function validRepo(repo){
  return typeof repo==='string'&&REPO.test(repo)&&repo.split('/').every(segment=>segment!=='.'&&segment!=='..');
}
function filePath(path){
  return typeof path==='string'&&path.length<=512&&path.toLowerCase().endsWith('.gguf')&&
    path.split('/').every(part=>part.length>0&&part.length<=128&&part!=='.'&&part!=='..'&&
      /^[a-zA-Z0-9_. -]+$/.test(part));
}
function hash(value){return typeof value==='string'&&SHA.test(value);}
export function huggingFaceDownloadUrl(repo,commit,path){
  if(!validRepo(repo)||!hash(commit)||!filePath(path))throw Error('huggingface: invalid repo, SHA or filename');
  return 'https://huggingface.co/'+repo.split('/').map(encodeURIComponent).join('/')+
    '/resolve/'+commit+'/'+path.split('/').map(encodeURIComponent).join('/')+'?download=true';
}
export async function listHuggingFaceGguf(repo,{fetchImpl=globalThis.fetch.bind(globalThis),signal}={}){
  if(!validRepo(repo))throw Error('huggingface: invalid repository ID');
  const controller=new AbortController();
  const handle=setTimeout(()=>controller.abort(),15000);
  const onAbort=()=>controller.abort();
  signal?.addEventListener('abort',onAbort,{once:true});
  try{
    const response=await fetchImpl('https://huggingface.co/api/models/'+
      repo.split('/').map(encodeURIComponent).join('/')+'?files_metadata=true',{
        method:'GET',headers:{Accept:'application/json'},credentials:'omit',
        redirect:'error',cache:'no-store',signal:controller.signal
      });
    if(!response.ok)throw Error('huggingface: HTTP '+response.status+' (model private, gated or unavailable)');
    if(Number(response.headers?.get('Content-Length'))>MAX_HF_RESPONSE)throw Error('huggingface: response too large');
    const data=await response.text();
    if(new TextEncoder().encode(data).length>MAX_HF_RESPONSE)throw Error('huggingface: response too large');
    const model=JSON.parse(data);
    if(!hash(model.sha)||!Array.isArray(model.siblings)||model.siblings.length>10000){
      throw Error('huggingface: invalid repository metadata');
    }
    const files=model.siblings.filter(entry=>plain(entry)&&filePath(entry.rfilename)).map(entry=>{
      const path=entry.rfilename;
      const size=entry.size??entry.lfs?.size??null;
      if(size!==null&&(!Number.isSafeInteger(size)||size<=0))throw Error('huggingface: invalid model size');
      return {path,size_bytes:size,download_url:huggingFaceDownloadUrl(repo,model.sha,path)};
    }).sort((a,b)=>a.path.localeCompare(b.path));
    if(files.length>1000)throw Error('huggingface: too many GGUF files');
    return {repo_id:repo,commit_sha:model.sha.toLowerCase(),gated:Boolean(model.gated),files};
  } finally {
    clearTimeout(handle);
    signal?.removeEventListener('abort',onAbort);
  }
}
const utf8=new TextDecoder('utf-8',{fatal:true});
export function parseGgufHeader(bytes){
  if(!(bytes instanceof ArrayBuffer)||bytes.byteLength<24)throw Error('gguf: truncated header');
  const dv=new DataView(bytes);
  const u8=new Uint8Array(bytes);
  if(String.fromCharCode(...u8.slice(0,4))!=='GGUF')throw Error('gguf: invalid magic');
  const version=dv.getUint32(4,true);
  if(version!==2&&version!==3)throw Error('gguf: unsupported version');
  const read64=offset=>{
    if(offset+8>dv.byteLength)throw Error('gguf: truncated metadata');
    const n=dv.getBigUint64(offset,true);
    if(n>BigInt(Number.MAX_SAFE_INTEGER))throw Error('gguf: invalid count');
    return Number(n);
  };
  const tensorCount=read64(8),kvCount=read64(16);
  if(tensorCount>10000000||kvCount>65536)throw Error('gguf: excessive counts');
  let cursor=24,metadataIncomplete=false;
  const metadata={};
  const need=n=>{if(!Number.isSafeInteger(n)||n<0||cursor+n>dv.byteLength)throw Error('gguf: truncated metadata');};
  const number=(kind)=>{
    const readers={
      0:[1,()=>dv.getUint8(cursor)],1:[1,()=>dv.getInt8(cursor)],
      2:[2,()=>dv.getUint16(cursor,true)],3:[2,()=>dv.getInt16(cursor,true)],
      4:[4,()=>dv.getUint32(cursor,true)],5:[4,()=>dv.getInt32(cursor,true)],
      6:[4,()=>dv.getFloat32(cursor,true)],7:[1,()=>dv.getUint8(cursor)!==0],
      10:[8,()=>Number(dv.getBigUint64(cursor,true))],11:[8,()=>Number(dv.getBigInt64(cursor,true))],
      12:[8,()=>dv.getFloat64(cursor,true)]
    };
    const item=readers[kind];if(!item)throw Error('gguf: invalid metadata type');
    need(item[0]);const value=item[1]();cursor+=item[0];return value;
  };
  const string=()=>{
    need(8);const n=read64(cursor);cursor+=8;
    if(n>1024*1024)throw Error('gguf: string metadata limit');
    need(n);
    const value=utf8.decode(u8.subarray(cursor,cursor+n));cursor+=n;
    return value;
  };
  function value(kind,depth=0){
    if(kind===8)return string();
    if(kind===9){
      if(depth>1)throw Error('gguf: nested array limit');
      need(12);const inner=dv.getUint32(cursor,true);cursor+=4;
      const count=read64(cursor);cursor+=8;
      if(count>500000)throw Error('gguf: metadata array limit');
      for(let i=0;i<count;i++)value(inner,depth+1);
      return '[array '+count+']';
    }
    return number(kind);
  }
  for(let i=0;i<kvCount;i++){
    try{
      const key=string();
      need(4);
      const kind=dv.getUint32(cursor,true);cursor+=4;
      const v=value(kind);
      if(key.length<=128 && /^(general\.|llama\.|qwen|gemma\.|tokenizer\.ggml\.model)/.test(key)){
        metadata[key]=typeof v==='string'?v.slice(0,256):v;
      }
    }catch(error){
      if(/truncated metadata|string metadata limit|metadata array limit/.test(error.message)){
        metadataIncomplete=true;break;
      }
      throw error;
    }
  }
  return {version,tensor_count:tensorCount,metadata_count:kvCount,metadata,metadata_incomplete:metadataIncomplete};
}
export async function inspectLocalGguf(file){
  if(!file||typeof file.name!=='string'||!file.name.toLowerCase().endsWith('.gguf')||
      !Number.isSafeInteger(file.size)||file.size<24||typeof file.slice!=='function'){
    throw Error('gguf: select a real .gguf file');
  }
  const buffer=await file.slice(0,Math.min(file.size,MAX_GGUF_HEADER)).arrayBuffer();
  const header=parseGgufHeader(buffer);
  return {filename:file.name.split(/[\\/]/).pop().slice(0,128),size_bytes:file.size,header};
}
export function makeModelProfile(source,settings=defaultSettings()){
  if(!plain(source))throw Error('profile: invalid source');
  const profile={schema_version:1,source,settings};
  validateModelProfile(profile);
  return profile;
}
export function validateModelProfile(profile){
  fields(profile,['schema_version','source','settings']);
  if(profile.schema_version!==1)throw Error('profile: unsupported schema');
  const source=profile.source;
  if(!plain(source)||!['api','huggingface','file'].includes(source.kind))throw Error('profile: invalid source');
  if(source.kind==='api'){
    fields(source,['kind','provider','model_id']);
    if(!['openai','anthropic','google','openrouter','groq','mistral','deepseek','cohere','huggingface'].includes(source.provider)||
        typeof source.model_id!=='string'||!source.model_id.trim()||source.model_id.length>160||
        !/^[A-Za-z0-9_./:-]+$/.test(source.model_id)){
      throw Error('profile: invalid API model identifier');
    }
  }else if(source.kind==='huggingface'){
    fields(source,['kind','repo_id','commit_sha','path','size_bytes']);
    huggingFaceDownloadUrl(source.repo_id,source.commit_sha,source.path);
    if(source.size_bytes!==null&&(!Number.isSafeInteger(source.size_bytes)||source.size_bytes<=0))
      throw Error('profile: invalid HF file size');
  }else{
    fields(source,['kind','filename','size_bytes','gguf_version']);
    if(!filePath(source.filename)||source.filename.includes('/')||!Number.isSafeInteger(source.size_bytes)||
        source.size_bytes<24||![2,3].includes(source.gguf_version))throw Error('profile: invalid local GGUF');
  }
  validateSettings(profile.settings);
  const serialized=JSON.stringify(profile);
  if(new TextEncoder().encode(serialized).length>MAX_JSON)throw Error('profile: too large');
  return structuredClone(profile);
}
function storageKey(tenant,projectId){
  if(!id(tenant)||!id(projectId))throw Error('profile: invalid project identity');
  return 'ocaf.builder.model.v1:'+tenant+':'+projectId;
}
export function saveModelProfile(storage,tenant,projectId,profile){
  const checked=validateModelProfile(profile);
  storage.setItem(storageKey(tenant,projectId),JSON.stringify(checked));
  return checked;
}
export function loadModelProfile(storage,tenant,projectId){
  const raw=storage.getItem(storageKey(tenant,projectId));
  if(raw===null)return null;
  if(raw.length>MAX_JSON)throw Error('profile: stored data too large');
  return validateModelProfile(JSON.parse(raw));
}
export function exportModelProfile(profile){
  return JSON.stringify(validateModelProfile(profile),null,2);
}
