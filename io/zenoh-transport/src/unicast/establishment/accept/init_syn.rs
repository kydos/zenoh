//
// Copyright (c) 2022 ZettaScale Technology
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
use super::AResult;
use crate::TransportManager;
use zenoh_link::LinkUnicast;
use zenoh_protocol::{
    core::{Resolution, WhatAmI, ZenohId},
    transport::{close, TransportBody},
};
use zenoh_result::zerror;

/*************************************/
/*             ACCEPT                */
/*************************************/

// Read and eventually accept an InitSyn
pub(super) struct Output {
    pub(super) whatami: WhatAmI,
    pub(super) zid: ZenohId,
    pub(super) resolution: Resolution,
    pub(super) batch_size: u16,
    pub(super) is_qos: bool,
}

pub(super) async fn recv(link: &LinkUnicast, manager: &TransportManager) -> AResult<Output> {
    // Wait to read an InitSyn
    let mut messages = link
        .read_transport_message()
        .await
        .map_err(|e| (e, Some(close::reason::INVALID)))?;
    if messages.len() != 1 {
        let e = zerror!(
            "Received multiple messages instead of a single InitSyn on {}: {:?}",
            link,
            messages,
        );
        return Err((e.into(), Some(close::reason::INVALID)));
    }

    let msg = messages.remove(0);
    let init_syn = match msg.body {
        TransportBody::InitSyn(init_syn) => init_syn,
        _ => {
            let e = zerror!(
                "Received invalid message instead of an InitSyn on {}: {:?}",
                link,
                msg.body
            );
            return Err((e.into(), Some(close::reason::INVALID)));
        }
    };

    // // Check the peer id associate to the authenticated link
    // match auth_link.peer_id {
    //     Some(zid) => {
    //         if zid != init_syn.zid {
    //             let e = zerror!(
    //                 "Inconsistent ZenohId in InitSyn on {}: {:?} {:?}",
    //                 link,
    //                 zid,
    //                 init_syn.zid
    //             );
    //             return Err((e.into(), Some(close::reason::INVALID)));
    //         }
    //     }
    //     None => auth_link.peer_id = Some(init_syn.zid),
    // }

    // Check if the version is supported
    if init_syn.version != manager.config.version {
        let e = zerror!(
            "Rejecting InitSyn on {} because of unsupported Zenoh version from peer: {}",
            link,
            init_syn.zid
        );
        return Err((e.into(), Some(close::reason::INVALID)));
    }

    // // Validate the InitSyn with the peer authenticators
    // let init_syn_properties: EstablishmentProperties = match msg.attachment.take() {
    //     Some(att) => EstablishmentProperties::try_from(&att)
    //         .map_err(|e| (e, Some(close::reason::INVALID)))?,
    //     None => EstablishmentProperties::new(),
    // };

    let output = Output {
        whatami: init_syn.whatami,
        zid: init_syn.zid,
        resolution: init_syn.resolution,
        batch_size: init_syn.batch_size,
        is_qos: init_syn.qos.is_some(),
    };
    Ok(output)
}
