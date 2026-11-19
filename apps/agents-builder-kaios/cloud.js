/* Same-origin HTTPS Builder Cloud transport for KaiOS canonical graph. */
(function(root){
  'use strict';
  var LIMIT=131072;
  function id(value){
    if(typeof value!=='string'||!/^[A-Za-z0-9_.-]{1,64}$/.test(value)||value==='.'||value==='..') throw Error('Invalid Cloud identifier');
    return value;
  }
  function fields(v,names){
    return v!==null&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).length===names.length&&names.every(function(k){return Object.prototype.hasOwnProperty.call(v,k);});
  }
  function head(v){
    if(!fields(v,['revision','sha256'])||typeof v.revision!=='number'||v.revision<1||v.revision>9007199254740991||Math.floor(v.revision)!==v.revision||typeof v.sha256!=='string'||!/^[0-9a-f]{64}$/.test(v.sha256)) throw Error('Invalid Cloud revision receipt');
    return v;
  }
  function same(a,b){return Array.isArray(a)&&a.length===b.length&&a.every(function(v,i){return v===b[i];});}
  function project(graph,v,tenant,pid){
    if(!v||v.tenant_id!==tenant||v.project_id!==pid)throw Error('Cloud project identity mismatch');
    return graph.compile(v,tenant);
  }
  function createClient(env,tenant,token,graph){
    id(tenant);
    if(typeof token!=='string'||!/^[\x21-\x7e]{1,4089}$/.test(token))throw Error('Invalid OAuth token');
    var origin=env.location&&env.location.origin;
    if(!origin||!/^https:\/\/[A-Za-z0-9.-]+(?::[0-9]{1,5})?$/.test(origin)||env.location.protocol!=='https:'||typeof env.fetch!=='function'||typeof env.TextDecoder!=='function')throw Error('Cloud requires the same-origin HTTPS gateway');
    var active=true, pending=null;
    function request(method,path,payload){
      if(!active)return Promise.reject(Error('Cloud disconnected'));
      if(pending)return Promise.reject(Error('Cloud request already in progress'));
      var body=payload===undefined?undefined:JSON.stringify(payload);
      if(body!==undefined)graph.parseUniqueJson(body,LIMIT);
      var ctl=typeof env.AbortController==='function'?new env.AbortController():null;
      var cancelled=false,reader=null,rejectCancel;
      var cancellation=new Promise(function(_,reject){rejectCancel=reject;});
      function cancelReader(){
        if(!reader)return;
        var current=reader;reader=null;
        Promise.resolve().then(function(){return current.cancel();}).catch(function(){});
      }
      function cancel(reason){
        if(cancelled)return;
        cancelled=true;
        rejectCancel(reason||Error('Cloud disconnected'));
        if(ctl)ctl.abort();
        cancelReader();
      }
      pending=cancel;
      function live(){if(!active||cancelled)throw Error('Cloud disconnected');}
      var options={method:method,credentials:'omit',redirect:'error',cache:'no-store',headers:{Authorization:'Bearer '+token,Accept:'application/json'}};
      if(body!==undefined){options.body=body;options.headers['Content-Type']='application/json';}
      if(ctl)options.signal=ctl.signal;
      var url=origin+path,timer;
      timer=env.setTimeout(function(){active=false;token='';cancel(Error('Cloud timeout; reconnect and reload before retrying a write'));},10000);
      var task=Promise.resolve().then(function(){live();return env.fetch(url,options);}).then(function(r){
        live();
        if(r.redirected||(r.url&&r.url!==url))throw Error('Cloud redirect denied');
        if(!r.ok){var e=Error('Cloud HTTP '+r.status);e.status=r.status;throw e;}
        if(!/^application\/json(?:\s*;|$)/i.test(r.headers.get('Content-Type')||''))throw Error('Invalid Cloud content type');
        var l=r.headers.get('Content-Length'),encoding=r.headers.get('Content-Encoding');
        if(l!==null&&(!/^[0-9]{1,9}$/.test(l)||Number(l)>LIMIT))throw Error('Cloud response limit');
        if(r.body&&typeof r.body.getReader==='function'){
          reader=r.body.getReader();var total=0,chunks=[];
          function reject(message){cancelReader();throw Error(message); }
          function next(){
            if(!active||cancelled)return reject('Cloud disconnected');
            return reader.read().then(function(part){
              if(!active||cancelled)return reject('Cloud disconnected');
              if(part.done){var bytes=new Uint8Array(total),offset=0;chunks.forEach(function(c){bytes.set(c,offset);offset+=c.length;});return bytes;}
              if(!(part.value instanceof Uint8Array)||chunks.length>=4096)return reject('Cloud response chunk limit');
              total+=part.value.byteLength;if(total>LIMIT)return reject('Cloud response limit');chunks.push(part.value);return next();
            });
          }
          return next();
        }
        // Gecko versions without streaming must have a bounded identity response.
        if(l===null||(encoding&&encoding.toLowerCase()!=='identity')||typeof r.arrayBuffer!=='function')throw Error('Cloud requires bounded identity response on this engine');
        return r.arrayBuffer().then(function(b){if(b.byteLength!==Number(l)||b.byteLength>LIMIT)throw Error('Cloud response length mismatch');return new Uint8Array(b);});
      }).then(function(bytes){live();return graph.parseUniqueJson(new env.TextDecoder('utf-8',{fatal:true}).decode(bytes),LIMIT);});
      function finish(){env.clearTimeout(timer);pending=null;}
      return Promise.race([task,cancellation]).then(function(v){finish();return v;},function(e){finish();throw e;});
    }
    return {
      disconnect:function(){active=false;token='';if(pending)pending();},
      load:function(pid){id(pid);return request('GET','/v1/projects/'+encodeURIComponent(pid)).then(function(v){
        if(!fields(v,['project','head']))throw Error('Invalid Cloud project response');
        project(graph,v.project,tenant,pid);head(v.head);
        if(v.project.revision!==v.head.revision)throw Error('Cloud revision mismatch');return v;
      });},
      save:function(p,expected){project(graph,p,tenant,id(p.project_id));if(expected!==null)head(expected);
        if(p.revision!==(expected===null?1:expected.revision+1))throw Error('Cloud CAS revision mismatch');
        var revision=p.revision;return request('PUT','/v1/projects/'+encodeURIComponent(p.project_id),{project:p,expected:expected}).then(function(v){head(v);if(v.revision!==revision)throw Error('Cloud write receipt mismatch');return v;});},
      preview:function(p){var plan=project(graph,p,tenant,id(p.project_id));return request('POST','/v1/projects/preview',p).then(function(v){
        if(!fields(v,['ordered_nodes','requested_tools','max_cost_minor','project_sha256'])||!same(v.ordered_nodes,plan.ordered_nodes)||!same(v.requested_tools,plan.requested_tools)||v.max_cost_minor!==plan.max_cost_minor||typeof v.project_sha256!=='string'||!/^[0-9a-f]{64}$/.test(v.project_sha256))throw Error('Cloud plan mismatch');return v;
      });}
    };
  }
  root.OCAFKaiosCloud={createClient:createClient,identifier:id};
}(typeof module!=='undefined'&&module.exports?module.exports:this));
