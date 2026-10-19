//! Enforces the OCAF rule: Markdown or Make prompts do not substitute for Rust code.
//! Structural checks are not a production security certification.
#![forbid(unsafe_code)]
use std::collections::HashSet;
use std::fs;
use std::path::{Component,Path,PathBuf};

pub const HEADER:&str="id\tkind\tpath\ttransport\tstatus";
const KINDS:&[&str]=&["agent","swarm","system","omat","omas"];
fn prefix(kind:&str)->&'static str {
 match kind { "agent"=>"agents","swarm"=>"swarms","system"=>"systems","omat"=>"omat","omas"=>"omas",_=>"INVALID" }
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Record {
 pub id:String,pub kind:String,pub path:PathBuf,pub transport:String,pub status:String
}
pub fn parse_registry(text:&str)->Result<Vec<Record>,Vec<String>>{
 let mut faults=Vec::new();
 let mut found=Vec::new();
 let mut ids=HashSet::new();
 let mut paths=HashSet::new();
 let mut lines=text.lines();
 if lines.next()!=Some(HEADER){return Err(vec!["wrong or absent registry header".into()]);}
 for (idx,line) in lines.enumerate(){
  if line.is_empty()||line.starts_with('#'){continue;}
  let parts:Vec<&str>=line.split('\t').collect();
  if parts.len()!=5{faults.push(format!("line {}: require 5 fields",idx+2));continue;}
  let (id,kind,path,transport,status)=(parts[0],parts[1],parts[2],parts[3],parts[4]);
  if id.is_empty()||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_')
    ||!KINDS.contains(&kind)||!["zenoh","none"].contains(&transport)
    ||!["PLANNED","IMPLEMENTED_UNVERIFIED","VERIFIED"].contains(&status){
   faults.push(format!("line {}: invalid fields",idx+2));continue;
  }
  let p=Path::new(path);
  if p.is_absolute()||p.components().count()<2
    ||p.components().any(|c|!matches!(c,Component::Normal(_)))
    ||p.components().next().unwrap().as_os_str()!=std::ffi::OsStr::new(prefix(kind)){
   faults.push(format!("line {}: unsafe or misplaced directory",idx+2));continue;
  }
  if !ids.insert(id.to_owned())||!paths.insert(p.to_path_buf()){
   faults.push(format!("line {}: duplicate component or folder",idx+2));continue;
  }
  found.push(Record{id:id.into(),kind:kind.into(),path:p.to_path_buf(),transport:transport.into(),status:status.into()});
 }
 if found.is_empty(){faults.push("empty component registry".into());}
 if faults.is_empty(){Ok(found)}else{Err(faults)}
}
fn read(path:&Path)->Option<String>{fs::read_to_string(path).ok()}
fn gather(dir:&Path,out:&mut Vec<PathBuf>){
 let Ok(entries)=fs::read_dir(dir) else{return;};
 for entry in entries.flatten(){
  let Ok(ft)=entry.file_type() else{continue;};
  if ft.is_symlink(){continue;}
  let p=entry.path();
  if ft.is_dir(){gather(&p,out);}
  else if ft.is_file()&&p.extension().is_some_and(|e|e=="rs"){out.push(p);}
 }
}
pub fn verify_component(repo:&Path,c:&Record)->Vec<String>{
 if c.status=="PLANNED"{return vec![];}
 let mut e=Vec::new();
 let dir=repo.join(&c.path);
 let canonical_repo=fs::canonicalize(repo).ok();
 let canonical_dir=fs::canonicalize(&dir).ok();
 if canonical_repo.is_none()||canonical_dir.as_ref().is_none_or(|p|!p.starts_with(canonical_repo.as_ref().unwrap())){
  return vec![format!("{}: missing/escaping own source folder",c.id)];
 }
 let cargo=read(&dir.join("Cargo.toml")).unwrap_or_default();
 if !cargo.contains("[package]")||!cargo.contains("edition"){
  e.push(format!("{}: missing valid Rust Cargo.toml",c.id));
 }
 let mut source_files=Vec::new();
 gather(&dir.join("src"),&mut source_files);
 let source=source_files.iter().filter_map(|p|read(p)).collect::<String>();
 if source.trim().len()<100||source.contains(&["todo","!()"].concat())||source.contains(&["unimplemented","!()"].concat()){
  e.push(format!("{}: missing actual Rust implementation, or placeholder code",c.id));
 }
 let mut test_files=Vec::new();
 gather(&dir.join("tests"),&mut test_files);
 let test_exists=test_files.iter().filter_map(|p|read(p))
    .any(|s|s.len()>60&&(s.contains("#[test]")||s.contains("#[tokio::test]")));
 if !test_exists{e.push(format!("{}: missing executable Rust tests",c.id));}
 if c.transport=="zenoh" {
  if !cargo.lines().any(|x|x.trim_start().starts_with("zenoh =")||x.trim_start().starts_with("zenoh="))
   ||!(source.contains("zenoh::")||source.contains("use zenoh")){
   e.push(format!("{}: missing required actual Zenoh dependency/API",c.id));
  }
 }
 for file in ["TECHNICAL.md","USER_GUIDE.md","CERTIFICATION.md"]{
  if read(&dir.join("docs").join(file)).is_none_or(|s|s.trim().len()<90){
   e.push(format!("{}: missing substantive documentation {}",c.id,file));
  }
 }
 if c.status=="VERIFIED" {
  // These fields are only an evidence inventory. CI and operator evidence must
  // still be checked independently against the actual systems.
  let cert=read(&dir.join("docs/CERTIFICATION.md")).unwrap_or_default();
  for marker in ["commit_sha=","ci_run_id=","review_id=","operator_approved=true"]{
   if !cert.lines().any(|line|line.trim_start().starts_with(marker)){
    e.push(format!("{}: VERIFIED lacks evidence field {}",c.id,marker));
   }
  }
 }
 e
}
fn find_cargo_packages(root:&Path,out:&mut Vec<PathBuf>){
 let Ok(entries)=fs::read_dir(root) else{return;};
 for entry in entries.flatten(){
  let Ok(kind)=entry.file_type() else{continue;};
  if kind.is_symlink()||!kind.is_dir(){continue;}
  let dir=entry.path();
  if dir.join("Cargo.toml").is_file(){out.push(dir);}
  else{find_cargo_packages(&dir,out);}
 }
}
pub fn audit(repo:&Path,registry:&Path)->Vec<String>{
 let content=match read(registry){Some(s)=>s,None=>return vec!["cannot open component registry".into()]};
 let entries=match parse_registry(&content){Ok(rows)=>rows,Err(errors)=>return errors};
 let mut errors=Vec::new();
 let registered:HashSet<PathBuf>=entries.iter().map(|x|x.path.clone()).collect();
 if entries.iter().filter(|x|x.kind=="swarm").count()<33
   ||entries.iter().filter(|x|x.kind=="agent").count()<132{
  errors.push("registry is missing one or more of 33 swarm folders and 132 role folders".into());
 }
 for kind in KINDS{
  let mut found=Vec::new();
  find_cargo_packages(&repo.join(prefix(kind)),&mut found);
  for dir in found {
   if dir.strip_prefix(repo).is_ok_and(|p|!registered.contains(p)){
    errors.push(format!("unregistered Rust component: {}",dir.display()));
   }
  }
 }
 for r in &entries{errors.extend(verify_component(repo,r));}
 errors
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn traversal_rejected(){
  let t=format!("{HEADER}\nX\tagent\tagents/../outside\tnone\tVERIFIED\n");
  assert!(parse_registry(&t).is_err());
 }
 #[test]fn duplicate_folder_rejected(){
  let t=format!("{HEADER}\nX\tagent\tagents/x\tnone\tPLANNED\nY\tagent\tagents/x\tnone\tPLANNED\n");
  assert!(parse_registry(&t).is_err());
 }
 #[test]fn planned_does_not_claim_impl(){
  let t=format!("{HEADER}\nX\tagent\tagents/x\tzenoh\tPLANNED\n");
  let r=parse_registry(&t).unwrap();
  assert!(verify_component(Path::new("/nonexistent"),&r[0]).is_empty());
 }
}
