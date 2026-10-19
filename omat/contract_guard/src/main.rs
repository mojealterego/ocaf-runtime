//! Offline OCAF Rust component structural gate.
#![forbid(unsafe_code)]
use std::path::PathBuf;
fn main(){
 let mut root=PathBuf::from(".");
 let mut registry=PathBuf::from("docs/agents/component_registry.tsv");
 let mut args=std::env::args().skip(1);
 while let Some(key)=args.next(){
  match key.as_str(){
   "--repo-root"=>root=PathBuf::from(args.next().unwrap_or_default()),
   "--registry"=>registry=PathBuf::from(args.next().unwrap_or_default()),
   "--help"|"-h"=>{println!("ocaf-agent-contract-guard --repo-root DIR --registry PATH");return;},
   _=>{eprintln!("unsupported argument {key}");std::process::exit(2);}
  }
 }
 if registry.is_relative(){registry=root.join(registry);}
 let problems=ocaf_agent_contract_guard::audit(&root,&registry);
 if problems.is_empty(){println!("OCAF STRUCTURE: PASS (NOT a production certification)");}
 else{for p in &problems{eprintln!("CONTRACT ERROR: {p}");}std::process::exit(1);}
}
