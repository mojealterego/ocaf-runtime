use ocaf_runtime::trs_dtn_sqlite::{claim,initialize};
use rusqlite::{params,Connection};

#[test]
fn multi_megabyte_corrupt_blob_is_rejected_atomically() {
    let mut db=Connection::open_in_memory().unwrap();
    initialize(&db).unwrap();
    db.execute(
        "INSERT INTO trs_dtn_bundles(tenant,id,ciphertext,expires,hops) VALUES(?1,?2,?3,?4,?5)",
        params!["t",vec![17u8;32],vec![3u8;8*1024*1024],"00000000000000001000",3],
    ).unwrap();
    assert!(claim(&mut db,"t","p",1,10,1).is_err());
    let leases:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_delivery",[],|row|row.get(0)).unwrap();
    assert_eq!(leases,0);
    let watermark:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_clock",[],|row|row.get(0)).unwrap();
    assert_eq!(watermark,0);
}
