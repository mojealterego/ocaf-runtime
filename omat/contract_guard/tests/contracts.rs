use ocaf_agent_contract_guard::{parse_registry,verify_component,Record,HEADER};
use std::{fs,path::PathBuf,sync::atomic::{AtomicUsize,Ordering}};
static ID:AtomicUsize=AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture{
 fn new()->Self{
  let id=ID.fetch_add(1,Ordering::SeqCst);
  let root=std::env::temp_dir().join(format!("ocaf_guard_{}_{}",std::process::id(),id));
  fs::create_dir_all(&root).unwrap();Self(root)
 }
 fn write(&self,path:&str,data:&str){
  let p=self.0.join(path);
  fs::create_dir_all(p.parent().unwrap()).unwrap();
  fs::write(p,data).unwrap();
 }
 fn record(&self,status:&str,transport:&str)->Record{
  Record{id:"G01".into(),kind:"swarm".into(),path:PathBuf::from("swarms/g01"),
   transport:transport.into(),status:status.into()}
 }
 fn valid(&self){
  self.write("swarms/g01/Cargo.toml","[package]\nname=\"g01\"\nversion=\"0.1.0\"\nedition=\"2021\"\n");
  self.write("swarms/g01/src/lib.rs","pub fn checked_add(a:u64,b:u64)->Result<u64,&'static str>{a.checked_add(b).ok_or(\"overflow\")}\n");
  self.write("swarms/g01/tests/contract.rs","#[test]\nfn overflow_rejected(){ assert!(g01::checked_add(u64::MAX,1).is_err()); }\n");
  for file in ["TECHNICAL.md","USER_GUIDE.md","CERTIFICATION.md"]{
   self.write(&format!("swarms/g01/docs/{file}"),
    "# OCAF test documentation\nThis is an explicitly synthetic non-production fixture with no certification. It exists to test the Rust file-structure validator only.\n");
  }
 }
}
impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
#[test]fn markdown_only_is_rejected(){
 let f=Fixture::new();
 f.write("swarms/g01/docs/TECHNICAL.md","# Markdown is not an implementation");
 let err=verify_component(&f.0,&f.record("IMPLEMENTED_UNVERIFIED","none"));
 assert!(err.iter().any(|x|x.contains("Rust implementation")));
 assert!(err.iter().any(|x|x.contains("Cargo.toml")));
}
#[test]fn implemented_rust_structure_passes(){
 let f=Fixture::new();f.valid();
 assert!(verify_component(&f.0,&f.record("IMPLEMENTED_UNVERIFIED","none")).is_empty());
}
#[test]fn required_zenoh_cannot_be_documentation_only(){
 let f=Fixture::new();f.valid();
 let e=verify_component(&f.0,&f.record("IMPLEMENTED_UNVERIFIED","zenoh"));
 assert!(e.iter().any(|x|x.contains("Zenoh")));
}
#[test]fn verified_requires_ci_and_operator_receipts(){
 let f=Fixture::new();f.valid();
 let e=verify_component(&f.0,&f.record("VERIFIED","none"));
 assert!(e.iter().any(|x|x.contains("ci_run_id=")));
 assert!(e.iter().any(|x|x.contains("operator_approved=true")));
}
#[test]fn duplicate_or_traversal_is_rejected(){
 let t=format!("{HEADER}\nX\tagent\tagents/a\tnone\tPLANNED\nY\tagent\tagents/a\tnone\tPLANNED\n");
 assert!(parse_registry(&t).is_err());
}
