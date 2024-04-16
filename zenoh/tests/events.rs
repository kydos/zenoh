//
// Copyright (c) 2023 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use std::time::Duration;
use zenoh::internal::ztimeout;
use zenoh::prelude::r#async::*;

const TIMEOUT: Duration = Duration::from_secs(10);

async fn open_session(listen: &[&str], connect: &[&str]) -> Session {
    let mut config = peer();
    config.listen.endpoints = listen
        .iter()
        .map(|e| e.parse().unwrap())
        .collect::<Vec<_>>();
    config.connect.endpoints = connect
        .iter()
        .map(|e| e.parse().unwrap())
        .collect::<Vec<_>>();
    config.scouting.multicast.set_enabled(Some(false)).unwrap();
    println!("[  ][01a] Opening session");
    ztimeout!(zenoh::open(config).res_async()).unwrap()
}

async fn close_session(session: Session) {
    println!("[  ][01d] Closing session");
    ztimeout!(session.close().res_async()).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn zenoh_events() {
    let session = open_session(&["tcp/127.0.0.1:18447"], &[]).await;
    let zid = session.zid();
    let sub1 = session
        .declare_subscriber(format!("@/session/{zid}/transport/unicast/*"))
        .res()
        .await
        .unwrap();
    let sub2 = session
        .declare_subscriber(format!("@/session/{zid}/transport/unicast/*/link/*"))
        .res()
        .await
        .unwrap();

    let session2 = open_session(&["tcp/127.0.0.1:18448"], &["tcp/127.0.0.1:18447"]).await;
    let zid2 = session2.zid();

    let sample = ztimeout!(sub1.recv_async());
    assert!(sample.is_ok());
    let key_expr = sample.as_ref().unwrap().key_expr().as_str();
    assert!(key_expr.eq(&format!("@/session/{zid}/transport/unicast/{zid2}")));
    assert!(sample.as_ref().unwrap().kind() == SampleKind::Put);

    let sample = ztimeout!(sub2.recv_async());
    assert!(sample.is_ok());
    let key_expr = sample.as_ref().unwrap().key_expr().as_str();
    assert!(key_expr.starts_with(&format!("@/session/{zid}/transport/unicast/{zid2}/link/")));
    assert!(sample.as_ref().unwrap().kind() == SampleKind::Put);

    let replies: Vec<Reply> = ztimeout!(session
        .get(format!("@/session/{zid}/transport/unicast/*"))
        .res_async())
    .unwrap()
    .into_iter()
    .collect();
    assert!(replies.len() == 1);
    assert!(replies[0].result().is_ok());
    let key_expr = replies[0].result().unwrap().key_expr().as_str();
    assert!(key_expr.eq(&format!("@/session/{zid}/transport/unicast/{zid2}")));

    let replies: Vec<Reply> = ztimeout!(session
        .get(format!("@/session/{zid}/transport/unicast/*/link/*"))
        .res_async())
    .unwrap()
    .into_iter()
    .collect();
    assert!(replies.len() == 1);
    assert!(replies[0].result().is_ok());
    let key_expr = replies[0].result().unwrap().key_expr().as_str();
    assert!(key_expr.starts_with(&format!("@/session/{zid}/transport/unicast/{zid2}/link/")));

    close_session(session2).await;

    let sample = ztimeout!(sub1.recv_async());
    assert!(sample.is_ok());
    let key_expr = sample.as_ref().unwrap().key_expr().as_str();
    assert!(key_expr.eq(&format!("@/session/{zid}/transport/unicast/{zid2}")));
    assert!(sample.as_ref().unwrap().kind() == SampleKind::Delete);

    let sample = ztimeout!(sub2.recv_async());
    assert!(sample.is_ok());
    let key_expr = sample.as_ref().unwrap().key_expr().as_str();
    assert!(key_expr.starts_with(&format!("@/session/{zid}/transport/unicast/{zid2}/link/")));
    assert!(sample.as_ref().unwrap().kind() == SampleKind::Delete);

    sub2.undeclare().res().await.unwrap();
    sub1.undeclare().res().await.unwrap();
    close_session(session).await;
}
