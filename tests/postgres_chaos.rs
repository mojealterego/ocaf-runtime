//! PostgreSQL fault injection: kill leader DB session, promote standby,
//! retain shared replay and previously committed durable work.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::rpc::{PeerPolicy,RpcAction,RpcError,RpcGateway,RpcRequest,RpcReply,RpcStatus,SignedFrame};
use ocaf_runtime::runtime::OcafRuntime;

const KEY: [u8;32] = [0x91;32];
const NOW: u64 = 1_800_000_000_000;

fn dsn() -> Option<String> {
    let v=std::env::var("OCAF_TEST_PG_DSN").ok();
    assert!(v.is_some() || std::env::var("OCAF_REQUIRE_PG_TESTS").is_err());
    v
}
async fn gateway(url:&str,leader:Arc<PgLeader>)->RpcGateway {
    RpcGateway::new("coord",OcafRuntime::new(0),
        HashMap::from([("submitter".into(),
            PeerPolicy::new(KEY,vec!["echo".into()]).allow_job_submission())]),
        16).unwrap()
        .with_postgres_replay(url,true).await.unwrap()
        .with_postgres_coordinator(url,true).await.unwrap()
        .with_postgres_job_db(url,true).await.unwrap()
        .with_pg_leader(leader)
}
fn signed(action:RpcAction) -> Vec<u8> {
    let req=RpcRequest::new("submitter","coord",NOW,5000,action).unwrap();
    SignedFrame::seal(&req,&KEY).unwrap()
}
async fn invoke(g:&RpcGateway,msg:&[u8])->Result<RpcReply,RpcError> {
    SignedFrame::open(&g.handle_at(msg,NOW).await?,&KEY)
}

#[tokio::test]
async fn abruptly_terminated_leader_session_promotes_standby_without_replaying_work() {
    let Some(url)=dsn() else {return};
    let cluster=format!("chaos_{}",uuid::Uuid::new_v4().simple());
    let leader=Arc::new(PgLeader::connect(&url,&cluster,true).await.unwrap());
    let standby=Arc::new(PgLeader::connect(&url,&cluster,true).await.unwrap());
    assert!(leader.try_acquire().await.unwrap());
    assert!(!standby.try_acquire().await.unwrap());
    let first=gateway(&url,Arc::clone(&leader)).await;
    let second=gateway(&url,Arc::clone(&standby)).await;

    let job_id=uuid::Uuid::new_v4().to_string();
    let submission=signed(RpcAction::SubmitJob{
        id:job_id.clone(),tool:"echo".into(),
        args:serde_json::json!({"message":"one durable submission"}),
    });
    assert_eq!(invoke(&first,&submission).await.unwrap().status,RpcStatus::Ok);
    assert!(matches!(invoke(&second,&submission).await,Err(RpcError::Unavailable)));

    // Kill the leader's actual PostgreSQL backend (not a graceful release).
    let pid=leader.backend_pid().await.unwrap();
    let (client,connection)=tokio_postgres::connect(&url,tokio_postgres::NoTls).await.unwrap();
    let task=tokio::spawn(async move {let _=connection.await;});
    let killed:bool=client.query_one("SELECT pg_terminate_backend($1)",&[&pid])
        .await.unwrap().get(0);
    assert!(killed);
    let mut promoted=false;
    for _ in 0..80 {
        if standby.try_acquire().await.unwrap() {
            promoted=true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(promoted,"standby should become leader after original session death");
    assert!(matches!(invoke(&first,&signed(RpcAction::GetJob{id:job_id.clone()})).await,
        Err(RpcError::Unavailable)));
    assert!(matches!(invoke(&second,&submission).await,Err(RpcError::Replay)));
    let retrieved=invoke(&second,&signed(RpcAction::GetJob{id:job_id.clone()}))
        .await.unwrap();
    assert_eq!(retrieved.status,RpcStatus::Ok);
    assert_eq!(retrieved.output["id"],job_id);
    assert_eq!(retrieved.output["status"],"pending");
    let history=invoke(&second,&signed(RpcAction::GetJobAudit{
        id:job_id,limit:20,
    })).await.unwrap();
    assert_eq!(history.output.as_array().unwrap().len(),1);
    client.close().await.unwrap();
    task.abort();
    standby.release().await.unwrap();
}
