/** Session-only credential registry. Never serialize, store, log or export API secrets. */
export const PROVIDERS=Object.freeze([
  ['openai','OpenAI'],['anthropic','Anthropic'],['google','Google Gemini'],
  ['openrouter','OpenRouter'],['groq','Groq'],['mistral','Mistral'],
  ['deepseek','DeepSeek'],['cohere','Cohere'],['huggingface','Hugging Face']
]);
const ids=new Set(PROVIDERS.map(p=>p[0]));
export class ProviderSessionKeys {
  #keys=new Map();
  add(provider,credential){
    if(!ids.has(provider))throw Error('credentials: unsupported provider');
    if(typeof credential!=='string'||credential.length<4||credential.length>4096||
        /[\r\n\x00-\x1f\x7f]/.test(credential)||credential.trim()!==credential){
      throw Error('credentials: invalid credential');
    }
    this.#keys.set(provider,credential);
  }
  has(provider){return this.#keys.has(provider);}
  remove(provider){this.#keys.delete(provider);}
  clear(){this.#keys.clear();}
  identifiers(){return [...this.#keys.keys()].sort();}
  // Only a deliberate provider adapter can access a secret. UI never calls this.
  withCredential(provider,operation){
    if(!ids.has(provider)||typeof operation!=='function'||!this.#keys.has(provider))throw Error('credentials: missing credential');
    return operation(this.#keys.get(provider));
  }
}
