use ocaf_runtime::trs_dtn_sqlite::{claim,initialize};
use rusqlite::{params,Connection};

#[test]
fn corrupted_empty_ciphertext_is_never_forwarded() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 db.execute("INSERT INTO trs_dtn_bundles(tenant,id,ciphertext,expires,hops) VALUES(?1,?2,?3,?4,?5)",
 params!["t",vec![7u8;32],Vec::<u8>::new(),"00000000000000000100",3]).unwrap();
 assert!(claim(&mut db,"t","p",1,10,1).is_err());
 let n:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_delivery",[],|r|r.get(0)).unwrap();
 assert_eq!(n,0);
}
#[test]
fn corrupted_oversized_ciphertext_is_never_forwarded() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 db.execute("INSERT INTO trs_dtn_bundles(tenant,id,ciphertext,expires,hops) VALUES(?1,?2,?3,?4,?5)",
 params!["t",vec![8u8;32],vec![42u8;65537],"00000000000000000100",3]).unwrap();
 assert!(claim(&mut db,"t","p",1,10,1).is_err());
}
