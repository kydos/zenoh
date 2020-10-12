//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//

//! The crate of the zenoh API.
//!
//! See the [Zenoh] struct for details.
//!
//! # Quick start examples
//!
//! ### Put a key/value into zenoh
//! ```
//! use zenoh::*;
//! use std::convert::TryInto;
//!
//! #[async_std::main]
//! async fn main() {
//!     let zenoh = Zenoh::new(config::default()).await.unwrap();
//!     let workspace = zenoh.workspace(None).await.unwrap();
//!     workspace.put(
//!         &"/demo/example/hello".try_into().unwrap(),
//!         "Hello World!".into()
//!     ).await.unwrap();
//!     zenoh.close().await.unwrap();
//! }
//! ```
//!
//! ### Subscribe for keys/values changes from zenoh
//! ```no_run
//! use zenoh::*;
//! use futures::prelude::*;
//! use std::convert::TryInto;
//!
//! #[async_std::main]
//! async fn main() {
//!     let zenoh = Zenoh::new(config::default()).await.unwrap();
//!     let workspace = zenoh.workspace(None).await.unwrap();
//!     let mut change_stream =
//!         workspace.subscribe(&"/demo/example/**".try_into().unwrap()).await.unwrap();
//!     while let Some(change) = change_stream.next().await {
//!         println!(">> {:?} for {} : {:?} at {}",
//!             change.kind, change.path, change.value, change.timestamp
//!         )
//!     }
//!     change_stream.close().await.unwrap();
//!     zenoh.close().await.unwrap();
//! }
//! ```
//!
//! ### Get keys/values from zenoh
//! ```no_run
//! use zenoh::*;
//! use futures::prelude::*;
//! use std::convert::TryInto;
//!
//! #[async_std::main]
//! async fn main() {
//!     let zenoh = Zenoh::new(config::default()).await.unwrap();
//!     let workspace = zenoh.workspace(None).await.unwrap();
//!     let mut data_stream = workspace.get(&"/demo/example/**".try_into().unwrap()).await.unwrap();
//!     while let Some(data) = data_stream.next().await {
//!         println!(">> {} : {:?} at {}",
//!             data.path, data.value, data.timestamp
//!         )
//!     }
//!     zenoh.close().await.unwrap();
//! }
//! ```
#![doc(
    html_logo_url = "http://zenoh.io/img/zenoh-dragon.png",
    html_favicon_url = "http://zenoh.io/favicon-32x32.png",
    html_root_url = "https://eclipse-zenoh.github.io/zenoh/zenoh/"
)]
#![feature(async_closure)]

#[macro_use]
extern crate lazy_static;

use log::debug;

pub mod net;

use net::Session;
pub use net::{ZError, ZErrorKind, ZResult};
use zenoh_router::runtime::Runtime;

mod workspace;
pub use workspace::*;

mod properties;
pub use properties::Properties;
mod path;
pub use path::Path;
mod pathexpr;
pub use pathexpr::PathExpr;
mod selector;
pub use selector::Selector;
mod values;
pub use values::*;

pub mod config;
pub mod utils;

pub use zenoh_protocol::core::{Timestamp, TimestampID};

/// The zenoh client API.
pub struct Zenoh {
    session: Session,
}

impl Zenoh {
    /// Creates a zenoh API, establishing a zenoh-net session with discovered peers and/or routers.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration [Properties](Properties) for the zenoh session
    ///
    /// # Examples
    /// ```
    /// # async_std::task::block_on(async {
    /// use zenoh::*;
    ///
    /// let zenoh = Zenoh::new(config::default()).await.unwrap();
    /// # })
    /// ```
    ///
    /// # Configuration Properties
    ///
    /// [Properties](Properties) are a list of key/value pairs.
    /// See [config](config) for accepted values.
    ///
    /// # Examples
    /// ```
    /// # async_std::task::block_on(async {
    /// use zenoh::*;
    ///
    /// let mut config = config::peer();
    /// config.insert("local_routing".to_string(), "false".to_string());
    /// config.insert("peer".to_string(), "tcp/10.10.10.10:7447,tcp/11.11.11.11:7447".to_string());
    ///
    /// let zenoh = Zenoh::new(config).await.unwrap();
    /// # })
    /// ```
    pub async fn new(config: Properties) -> ZResult<Zenoh> {
        Ok(Zenoh {
            session: net::open(config.into()).await?,
        })
    }

    /// Creates a Zenoh API with an existing Runtime.
    /// This operation is used by the plugins to share the same Runtime than the router.
    #[doc(hidden)]
    pub async fn init(runtime: Runtime) -> Zenoh {
        Zenoh {
            session: Session::init(runtime, true).await,
        }
    }

    /// Returns the zenoh-net [Session](net::Session) used by this zenoh session.
    /// This is for advanced use cases requiring fine usage of the zenoh-net API.
    #[doc(hidden)]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Creates a [`Workspace`] with an optional [`Path`] as `prefix`.  
    /// All relative [`Path`] or [`Selector`] used with this Workspace will be relative to the
    /// specified prefix. Not specifying a prefix is equivalent to specifying "/" as prefix,
    /// meaning in this case that all relative paths/selectors will be prependend with "/".
    ///
    /// # Examples
    /// ```
    /// # async_std::task::block_on(async {
    /// use zenoh::*;
    /// use std::convert::TryInto;
    ///
    /// let zenoh = Zenoh::new(config::default()).await.unwrap();
    /// let workspace = zenoh.workspace(Some("/demo/example".try_into().unwrap())).await.unwrap();
    /// // The following it equivalent to a PUT on "/demo/example/hello".
    /// workspace.put(
    ///     &"hello".try_into().unwrap(),
    ///     "Hello World!".into()
    /// ).await.unwrap();
    /// # })
    /// ```
    pub async fn workspace(&self, prefix: Option<Path>) -> ZResult<Workspace<'_>> {
        debug!("New workspace with prefix: {:?}", prefix);
        Workspace::new(&self, prefix).await
    }

    /// Closes the zenoh API and the associated zenoh-net session.
    ///
    /// Note that on drop, the zenoh-net session is also automatically closed.
    /// But you may want to use this function to handle errors or
    /// close the session synchronously.
    ///
    /// # Examples
    /// ```
    /// # async_std::task::block_on(async {
    /// use zenoh::*;
    ///
    /// let zenoh = Zenoh::new(config::default()).await.unwrap();
    /// zenoh.close();
    /// # })
    /// ```
    pub async fn close(self) -> ZResult<()> {
        self.session.close().await
    }
}