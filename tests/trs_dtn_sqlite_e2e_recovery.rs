use ocaf_runtime::trs_dtn_sqlite::{acknowledge,claim,initialize,insert,purge_expired,release};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;

#[test]
fn durable_end_to_end_recovery_and_tenant_isolation() {
    let path=tempfile::NamedTempFile::new().unwrap();
    let a=ReliableBundle{id:[41;32],ciphertext:vec![10,11],expires_at:100,hops_remaining:3};
    let b=ReliableBundle{id:[41;32],ciphertext:vec![20,21],expires_at:120,hops_remaining:3};
    {
        let mut db=Connection::open(path.path()).unwrap();initialize(&db).unwrap();
        insert(&mut db,"alpha",&a,1,4,32).unwrap();
        insert(&mut db,"beta",&b,1,4,32).unwrap();
        let sent=claim(&mut db,"alpha","peer",2,10,1).unwrap();
        assert_eq!(sent.len(),1);
        assert_eq!(sent[0].ciphertext,a.ciphertext);
        assert_eq!(sent[0].hops_remaining,2);
    }
    {
        let mut db=Connection::open(path.path()).unwrap();initialize(&db).unwrap();
        assert!(claim(&mut db,"alpha","peer",5,10,1).unwrap().is_empty());
        assert_eq!(claim(&mut db,"beta","peer",5,10,1).unwrap()[0].ciphertext,b.ciphertext);
        assert!(acknowledge(&mut db,"alpha","peer",&a.id,13).is_err());
        let resent=claim(&mut db,"alpha","peer",13,10,1).unwrap();
        assert_eq!(resent.len(),1);
        acknowledge(&mut db,"alpha","peer",&a.id,14).unwrap();
        assert!(claim(&mut db,"alpha","peer",25,10,1).unwrap().is_empty());
        assert_eq!(claim(&mut db,"beta","peer",25,10,1).unwrap().len(),1);
        assert!(release(&mut db,"alpha","peer",&a.id).is_err());
        assert_eq!(purge_expired(&mut db,"alpha",100).unwrap(),1);
        assert_eq!(purge_expired(&mut db,"beta",100).unwrap(),0);
    }
}
