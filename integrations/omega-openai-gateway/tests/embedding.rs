use omega_openai_gateway::embedding::{EmbedRequest, parse_embedding};
use serde_json::json;

#[test]
fn fixed_model_and_text_preserved() {
    let r:EmbedRequest=serde_json::from_value(json!({"input":"  żółć\n"})).unwrap();
    assert_eq!(r.payload("configured").unwrap(),json!({"model":"configured","input":"  żółć\n","encoding_format":"float"}));
    assert!(serde_json::from_value::<EmbedRequest>(json!({"input":"x","url":"http://example.com"})).is_err());
}
#[test]
fn rejects_mismatched_dimension_model_and_invalid_vectors() {
    let valid=json!({"model":"emb","data":[{"index":0,"embedding":[0.5,1.0]}]});
    assert_eq!(parse_embedding(&valid,"emb",2).unwrap(),vec![0.5,1.0]);
    assert!(parse_embedding(&valid,"wrong",2).is_err());
    assert!(parse_embedding(&valid,"emb",3).is_err());
    for v in [json!([0,0]),json!([1e100,0]),json!([1,"x"])] {
        assert!(parse_embedding(&json!({"model":"emb","data":[{"index":0,"embedding":v}]}),"emb",2).is_err());
    }
}
#[cfg(unix)]
#[test]
fn private_database_rejects_symlinks_and_public_directories() {
    use omega_openai_gateway::private_db::prepare_private_database;
    use std::os::unix::fs::{symlink,PermissionsExt};
    let dir=tempfile::tempdir().unwrap();
    let private=dir.path().join("private");
    std::fs::create_dir(&private).unwrap();
    std::fs::set_permissions(&private,std::fs::Permissions::from_mode(0o700)).unwrap();
    let db=private.join("index.sqlite");
    assert_eq!(prepare_private_database(&db).unwrap(),db);
    symlink(&db,private.join("alias.sqlite")).unwrap();
    assert!(prepare_private_database(&private.join("alias.sqlite")).is_err());
    std::fs::set_permissions(&private,std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(prepare_private_database(&db).is_err());
}
