/**
 * OCAF session-bound local GGUF inference client.
 * This client is exclusively for the operator-owned loopback companion server.
 * It never contacts third-party providers and never persists credentials.
 */
import {validateSettings} from './models.mjs';
import {parseUniqueJson} from './strict-json.mjs';

const MODEL=/^[A-Za-z0-9][A-Za-z0-9_. -]{0,126}\.gguf$/i;

// Bound actual bytes before allocating the complete response, and reject
// ambiguous JSON using the same parser as the builder admission boundary.
async function readRunnerJson(response,maxBytes,upload=false){
  const limitError=()=>new Error(upload?'runner: upload response limit':'runner: response limit');
  const invalid=()=>new Error(upload?'runner: invalid upload response':'runner: invalid JSON response');
  const declared=response.headers?.get('Content-Length');
  if(declared!=null&&(!/^\d+$/.test(declared)||Number(declared)>maxBytes)){
    if(response.body)await response.body.cancel().catch(()=>{});
    throw limitError();
  }
  if(!response.body?.getReader)throw invalid();
  const reader=response.body.getReader();
  const bytes=new Uint8Array(maxBytes);
  let total=0;
  try{
    for(;;){
      const {done,value}=await reader.read();
      if(done)break;
      if(!(value instanceof Uint8Array))throw invalid();
      if(value.byteLength>maxBytes-total)throw limitError();
      bytes.set(value,total);total+=value.byteLength;
    }
  }catch(error){
    await reader.cancel().catch(()=>{});throw error;
  }finally{reader.releaseLock();}
  try{
    const text=new TextDecoder('utf-8',{fatal:true}).decode(bytes.subarray(0,total));
    const result=parseUniqueJson(text,{maxBytes});
    if(!result||typeof result!=='object'||Array.isArray(result))throw invalid();
    return result;
  }catch{throw invalid();}
}

export class LocalRunnerError extends Error {
  constructor(status,message){
    super(message);
    this.name='LocalRunnerError';
    this.status=status;
  }
}

export class LocalRunnerClient {
  #token;
  #origin;
  #fetch;
  constructor({token,origin=location.origin,fetchImpl=globalThis.fetch.bind(globalThis)}={}){
    const url=new URL(origin);
    if(url.protocol!=='http:'||!['127.0.0.1','localhost'].includes(url.hostname)||
       url.username||url.password||url.pathname!=='/'||url.search||url.hash){
      throw new Error('runner: a loopback http://127.0.0.1 origin is required');
    }
    if(typeof token!=='string'||token.length<8||token.length>512||
       token.trim()!==token||/[\x00-\x1f\x7f]/.test(token))throw new Error('runner: invalid session token');
    if(typeof fetchImpl!=='function')throw new Error('runner: fetch unavailable');
    this.#token=token;
    this.#origin=url.origin;
    this.#fetch=fetchImpl;
  }
  disconnect(){this.#token=null;}
  async #request(endpoint,{method='GET',payload=null,timeout=15000}={}){
    if(!this.#token)throw new Error('runner: disconnected');
    const controller=new AbortController();
    const timer=setTimeout(()=>controller.abort(),timeout);
    try{
      const init={method,headers:{Authorization:'Bearer '+this.#token,Accept:'application/json'},
        credentials:'omit',redirect:'error',cache:'no-store',
        referrerPolicy:'no-referrer',signal:controller.signal};
      if(payload!==null){
        const serialized=JSON.stringify(payload);
        if(serialized.length>32768)throw new Error('runner: request exceeds limit');
        init.headers['Content-Type']='application/json';
        init.body=serialized;
      }
      const res=await this.#fetch(this.#origin+'/v1/local-runner/'+endpoint,init);
      const result=await readRunnerJson(res,512*1024);
      if(!res.ok)throw new LocalRunnerError(res.status,
        typeof result?.error==='string'?result.error.slice(0,200):'runner: HTTP '+res.status);
      if(!result||typeof result!=='object'||Array.isArray(result))throw new Error('runner: invalid response');
      return result;
    }finally{clearTimeout(timer);}
  }
  async models(){
    const result=await this.#request('models');
    if(!Array.isArray(result.models)||result.models.length>200||
       result.models.some(m=>!MODEL.test(m?.name)||!Number.isSafeInteger(m?.size_bytes)||
         m.size_bytes<24||![2,3].includes(m?.version)))throw new Error('runner: invalid models response');
    return result.models;
  }
  async status(){
    const result=await this.#request('status');
    if(!['stopped','loading','ready','failed'].includes(result.state)||
       (result.model!==null&&typeof result.model!=='string'))
      throw new Error('runner: invalid status response');
    return result;
  }
  async start(filename,settings){
    if(typeof filename!=='string'||!MODEL.test(filename)||filename.includes('..'))throw new Error('runner: invalid model filename');
    return this.#request('start',{method:'POST',
      payload:{model:filename,settings:validateSettings(settings)},timeout:15000});
  }
  async chat(prompt){
    if(typeof prompt!=='string'||!prompt.length||prompt.length>12000)throw new Error('runner: invalid prompt');
    const result=await this.#request('chat',{method:'POST',payload:{prompt},timeout:190000});
    if(typeof result.text!=='string'||result.text.length>200000)throw new Error('runner: invalid chat response');
    return result;
  }
  async importGguf(file){
    if(!this.#token)throw new Error('runner: disconnected');
    if(!(file instanceof Blob)||typeof file.name!=='string'||!MODEL.test(file.name)||
      file.name.includes('..')||!Number.isSafeInteger(file.size)||
      file.size<24||file.size>100*1024**3)throw new Error('runner: invalid GGUF upload');
    const controller=new AbortController();
    const timeout=setTimeout(()=>controller.abort(),20*60*1000);
    try{
      const response=await this.#fetch(this.#origin+'/v1/local-runner/import',{
        method:'POST',body:file,
        credentials:'omit',redirect:'error',cache:'no-store',
        referrerPolicy:'no-referrer',signal:controller.signal,
        headers:{
          Authorization:'Bearer '+this.#token,
          Accept:'application/json',
          'Content-Type':'application/octet-stream',
          'X-OCAF-Model-Name':file.name,
        }
      });
      const body=await readRunnerJson(response,8192,true);
      if(!response.ok)throw new LocalRunnerError(response.status,
        typeof body.error==='string'?body.error.slice(0,200):'runner: upload failed');
      if(body.name!==file.name||body.size_bytes!==file.size||
        !/^[0-9a-f]{64}$/.test(body.sha256)||![2,3].includes(body.version))
        throw new Error('runner: invalid upload receipt');
      return body;
    }finally{clearTimeout(timeout);}
  }
  stop(){return this.#request('stop',{method:'POST',payload:{},timeout:15000});}
}
