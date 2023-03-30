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
use crate::{common::extension, RCodec, WCodec, Zenoh080, Zenoh080Condition, Zenoh080Header};
use zenoh_buffers::{
    reader::{DidntRead, Reader},
    writer::{DidntWrite, Writer},
};
use zenoh_protocol::{
    common::{iext, imsg, ZExtZ64},
    core::{ExprId, WireExpr},
    network::{
        declare::{self, keyexpr, queryable, subscriber, token, Declare, DeclareBody},
        id, Mapping,
    },
};

// Declaration
impl<W> WCodec<&DeclareBody, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &DeclareBody) -> Self::Output {
        match x {
            DeclareBody::DeclareKeyExpr(r) => self.write(&mut *writer, r)?,
            DeclareBody::UndeclareKeyExpr(r) => self.write(&mut *writer, r)?,
            DeclareBody::DeclareSubscriber(r) => self.write(&mut *writer, r)?,
            DeclareBody::UndeclareSubscriber(r) => self.write(&mut *writer, r)?,
            DeclareBody::DeclareQueryable(r) => self.write(&mut *writer, r)?,
            DeclareBody::UndeclareQueryable(r) => self.write(&mut *writer, r)?,
            DeclareBody::DeclareToken(r) => self.write(&mut *writer, r)?,
            DeclareBody::UndeclareToken(r) => self.write(&mut *writer, r)?,
        }

        Ok(())
    }
}

impl<R> RCodec<DeclareBody, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<DeclareBody, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        use declare::id::*;
        let d = match imsg::mid(codec.header) {
            D_KEYEXPR => DeclareBody::DeclareKeyExpr(codec.read(&mut *reader)?),
            U_KEYEXPR => DeclareBody::UndeclareKeyExpr(codec.read(&mut *reader)?),
            D_SUBSCRIBER => DeclareBody::DeclareSubscriber(codec.read(&mut *reader)?),
            U_SUBSCRIBER => DeclareBody::UndeclareSubscriber(codec.read(&mut *reader)?),
            D_QUERYABLE => DeclareBody::DeclareQueryable(codec.read(&mut *reader)?),
            U_QUERYABLE => DeclareBody::UndeclareQueryable(codec.read(&mut *reader)?),
            D_TOKEN => DeclareBody::DeclareToken(codec.read(&mut *reader)?),
            U_TOKEN => DeclareBody::UndeclareToken(codec.read(&mut *reader)?),
            _ => return Err(DidntRead),
        };

        Ok(d)
    }
}

// Declare
impl<W> WCodec<&Declare, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &Declare) -> Self::Output {
        // Header
        let mut header = id::DECLARE;
        let mut n_exts = ((x.ext_qos != declare::ext::QoSType::default()) as u8)
            + (x.ext_tstamp.is_some() as u8);
        if n_exts != 0 {
            header |= declare::flag::Z;
        }
        self.write(&mut *writer, header)?;

        // Extensions
        if x.ext_qos != declare::ext::QoSType::default() {
            n_exts -= 1;
            self.write(&mut *writer, (x.ext_qos, n_exts != 0))?;
        }
        if let Some(ts) = x.ext_tstamp.as_ref() {
            n_exts -= 1;
            self.write(&mut *writer, (ts, n_exts != 0))?;
        }

        // Body
        self.write(&mut *writer, &x.body)?;

        Ok(())
    }
}

impl<R> RCodec<Declare, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<Declare, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<Declare, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<Declare, Self::Error> {
        if imsg::mid(self.header) != id::DECLARE {
            return Err(DidntRead);
        }

        // Extensions
        let mut ext_qos = declare::ext::QoSType::default();
        let mut ext_tstamp = None;

        let mut has_ext = imsg::has_flag(self.header, declare::flag::Z);
        while has_ext {
            let ext: u8 = self.codec.read(&mut *reader)?;
            let eodec = Zenoh080Header::new(ext);
            match iext::eid(ext) {
                declare::ext::QoS::ID => {
                    let (q, ext): (declare::ext::QoSType, bool) = eodec.read(&mut *reader)?;
                    ext_qos = q;
                    has_ext = ext;
                }
                declare::ext::Timestamp::ID => {
                    let (t, ext): (declare::ext::TimestampType, bool) = eodec.read(&mut *reader)?;
                    ext_tstamp = Some(t);
                    has_ext = ext;
                }
                _ => {
                    has_ext = extension::skip(reader, "Declare", ext)?;
                }
            }
        }

        // Body
        let body: DeclareBody = self.codec.read(&mut *reader)?;

        Ok(Declare {
            body,
            ext_qos,
            ext_tstamp,
        })
    }
}

// DeclareKeyExpr
impl<W> WCodec<&keyexpr::DeclareKeyExpr, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &keyexpr::DeclareKeyExpr) -> Self::Output {
        // Header
        let mut header = declare::id::D_KEYEXPR;
        if x.wire_expr.has_suffix() {
            header |= keyexpr::flag::N;
        }
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;
        self.write(&mut *writer, &x.wire_expr)?;

        Ok(())
    }
}

impl<R> RCodec<keyexpr::DeclareKeyExpr, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<keyexpr::DeclareKeyExpr, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<keyexpr::DeclareKeyExpr, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<keyexpr::DeclareKeyExpr, Self::Error> {
        if imsg::mid(self.header) != declare::id::D_KEYEXPR {
            return Err(DidntRead);
        }

        let id: ExprId = self.codec.read(&mut *reader)?;
        let ccond = Zenoh080Condition::new(imsg::has_flag(self.header, keyexpr::flag::N));
        let wire_expr: WireExpr<'static> = ccond.read(&mut *reader)?;

        // Extensions
        let has_ext = imsg::has_flag(self.header, keyexpr::flag::Z);
        if has_ext {
            extension::skip_all(reader, "DeclareKeyExpr")?;
        }

        Ok(keyexpr::DeclareKeyExpr { id, wire_expr })
    }
}

// ForgetKeyExpr
impl<W> WCodec<&keyexpr::UndeclareKeyExpr, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &keyexpr::UndeclareKeyExpr) -> Self::Output {
        // Header
        let header = declare::id::U_KEYEXPR;
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;

        Ok(())
    }
}

impl<R> RCodec<keyexpr::UndeclareKeyExpr, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<keyexpr::UndeclareKeyExpr, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<keyexpr::UndeclareKeyExpr, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<keyexpr::UndeclareKeyExpr, Self::Error> {
        if imsg::mid(self.header) != declare::id::U_KEYEXPR {
            return Err(DidntRead);
        }

        let id: ExprId = self.codec.read(&mut *reader)?;

        // Extensions
        let has_ext = imsg::has_flag(self.header, keyexpr::flag::Z);
        if has_ext {
            extension::skip_all(reader, "ForgetKeyExpr")?;
        }

        Ok(keyexpr::UndeclareKeyExpr { id })
    }
}

// SubscriberInfo
crate::impl_zextz64!(subscriber::ext::SubscriberInfo, subscriber::ext::Info::ID);

// DeclareSubscriber
impl<W> WCodec<&subscriber::DeclareSubscriber, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &subscriber::DeclareSubscriber) -> Self::Output {
        // Header
        let mut header = declare::id::D_SUBSCRIBER;
        let mut n_exts = (x.ext_info != subscriber::ext::SubscriberInfo::default()) as u8;
        if n_exts != 0 {
            header |= subscriber::flag::Z;
        }
        if x.mapping != Mapping::default() {
            header |= subscriber::flag::M;
        }
        if x.wire_expr.has_suffix() {
            header |= subscriber::flag::N;
        }
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;
        self.write(&mut *writer, &x.wire_expr)?;

        // Extensions
        if x.ext_info != subscriber::ext::SubscriberInfo::default() {
            n_exts -= 1;
            self.write(&mut *writer, (x.ext_info, n_exts != 0))?;
        }

        Ok(())
    }
}

impl<R> RCodec<subscriber::DeclareSubscriber, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<subscriber::DeclareSubscriber, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<subscriber::DeclareSubscriber, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<subscriber::DeclareSubscriber, Self::Error> {
        if imsg::mid(self.header) != declare::id::D_SUBSCRIBER {
            return Err(DidntRead);
        }

        // Body
        let id: subscriber::SubscriberId = self.codec.read(&mut *reader)?;
        let ccond = Zenoh080Condition::new(imsg::has_flag(self.header, subscriber::flag::N));
        let wire_expr: WireExpr<'static> = ccond.read(&mut *reader)?;
        let mapping = if imsg::has_flag(self.header, subscriber::flag::M) {
            Mapping::Sender
        } else {
            Mapping::Receiver
        };

        // Extensions
        let mut ext_info = subscriber::ext::SubscriberInfo::default();

        let mut has_ext = imsg::has_flag(self.header, subscriber::flag::Z);
        while has_ext {
            let ext: u8 = self.codec.read(&mut *reader)?;
            let eodec = Zenoh080Header::new(ext);
            match iext::eid(ext) {
                subscriber::ext::Info::ID => {
                    let (i, ext): (subscriber::ext::SubscriberInfo, bool) =
                        eodec.read(&mut *reader)?;
                    ext_info = i;
                    has_ext = ext;
                }
                _ => {
                    has_ext = extension::skip(reader, "DeclareSubscriber", ext)?;
                }
            }
        }

        Ok(subscriber::DeclareSubscriber {
            id,
            wire_expr,
            mapping,
            ext_info,
        })
    }
}

// ForgetSubscriber
impl<W> WCodec<&subscriber::UndeclareSubscriber, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &subscriber::UndeclareSubscriber) -> Self::Output {
        // Header
        let header = declare::id::U_SUBSCRIBER;
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;

        Ok(())
    }
}

impl<R> RCodec<subscriber::UndeclareSubscriber, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<subscriber::UndeclareSubscriber, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<subscriber::UndeclareSubscriber, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<subscriber::UndeclareSubscriber, Self::Error> {
        if imsg::mid(self.header) != declare::id::U_SUBSCRIBER {
            return Err(DidntRead);
        }

        // Body
        let id: subscriber::SubscriberId = self.codec.read(&mut *reader)?;

        // Extensions
        let has_ext = imsg::has_flag(self.header, subscriber::flag::Z);
        if has_ext {
            extension::skip_all(reader, "ForgetSubscriber")?;
        }

        Ok(subscriber::UndeclareSubscriber { id })
    }
}

// QueryableInfo
crate::impl_zextz64!(queryable::ext::QueryableInfo, queryable::ext::Info::ID);

// DeclareQueryable
impl<W> WCodec<&queryable::DeclareQueryable, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &queryable::DeclareQueryable) -> Self::Output {
        // Header
        let mut header = declare::id::D_QUERYABLE;
        let mut n_exts = (x.ext_info != queryable::ext::QueryableInfo::default()) as u8;
        if n_exts != 0 {
            header |= subscriber::flag::Z;
        }
        if x.mapping != Mapping::default() {
            header |= subscriber::flag::M;
        }
        if x.wire_expr.has_suffix() {
            header |= subscriber::flag::N;
        }
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;
        self.write(&mut *writer, &x.wire_expr)?;
        if x.ext_info != queryable::ext::QueryableInfo::default() {
            n_exts -= 1;
            self.write(&mut *writer, (x.ext_info, n_exts != 0))?;
        }

        Ok(())
    }
}

impl<R> RCodec<queryable::DeclareQueryable, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<queryable::DeclareQueryable, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<queryable::DeclareQueryable, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<queryable::DeclareQueryable, Self::Error> {
        if imsg::mid(self.header) != declare::id::D_QUERYABLE {
            return Err(DidntRead);
        }

        // Body
        let id: queryable::QueryableId = self.codec.read(&mut *reader)?;
        let ccond = Zenoh080Condition::new(imsg::has_flag(self.header, queryable::flag::N));
        let wire_expr: WireExpr<'static> = ccond.read(&mut *reader)?;
        let mapping = if imsg::has_flag(self.header, queryable::flag::M) {
            Mapping::Sender
        } else {
            Mapping::Receiver
        };

        // Extensions
        let mut ext_info = queryable::ext::QueryableInfo::default();

        let mut has_ext = imsg::has_flag(self.header, queryable::flag::Z);
        while has_ext {
            let ext: u8 = self.codec.read(&mut *reader)?;
            let eodec = Zenoh080Header::new(ext);
            match iext::eid(ext) {
                queryable::ext::Info::ID => {
                    let (i, ext): (queryable::ext::QueryableInfo, bool) =
                        eodec.read(&mut *reader)?;
                    ext_info = i;
                    has_ext = ext;
                }
                _ => {
                    has_ext = extension::skip(reader, "DeclareQueryable", ext)?;
                }
            }
        }

        Ok(queryable::DeclareQueryable {
            id,
            wire_expr,
            mapping,
            ext_info,
        })
    }
}

// ForgetQueryable
impl<W> WCodec<&queryable::UndeclareQueryable, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &queryable::UndeclareQueryable) -> Self::Output {
        // Header
        let header = declare::id::U_QUERYABLE;
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;

        Ok(())
    }
}

impl<R> RCodec<queryable::UndeclareQueryable, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<queryable::UndeclareQueryable, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<queryable::UndeclareQueryable, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<queryable::UndeclareQueryable, Self::Error> {
        if imsg::mid(self.header) != declare::id::U_QUERYABLE {
            return Err(DidntRead);
        }

        // Body
        let id: queryable::QueryableId = self.codec.read(&mut *reader)?;

        // Extensions
        let has_ext = imsg::has_flag(self.header, queryable::flag::Z);
        if has_ext {
            extension::skip_all(reader, "ForgetQueryable")?;
        }

        Ok(queryable::UndeclareQueryable { id })
    }
}

// DeclareToken
impl<W> WCodec<&token::DeclareToken, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &token::DeclareToken) -> Self::Output {
        // Header
        let mut header = declare::id::D_TOKEN;
        if x.mapping != Mapping::default() {
            header |= subscriber::flag::M;
        }
        if x.wire_expr.has_suffix() {
            header |= subscriber::flag::N;
        }
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;
        self.write(&mut *writer, &x.wire_expr)?;

        Ok(())
    }
}

impl<R> RCodec<token::DeclareToken, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<token::DeclareToken, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);
        codec.read(reader)
    }
}

impl<R> RCodec<token::DeclareToken, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<token::DeclareToken, Self::Error> {
        if imsg::mid(self.header) != declare::id::D_TOKEN {
            return Err(DidntRead);
        }

        // Body
        let id: token::TokenId = self.codec.read(&mut *reader)?;
        let ccond = Zenoh080Condition::new(imsg::has_flag(self.header, token::flag::N));
        let wire_expr: WireExpr<'static> = ccond.read(&mut *reader)?;
        let mapping = if imsg::has_flag(self.header, token::flag::M) {
            Mapping::Sender
        } else {
            Mapping::Receiver
        };

        // Extensions
        let has_ext = imsg::has_flag(self.header, token::flag::Z);
        if has_ext {
            extension::skip_all(reader, "DeclareToken")?;
        }

        Ok(token::DeclareToken {
            id,
            wire_expr,
            mapping,
        })
    }
}

// ForgetToken
impl<W> WCodec<&token::UndeclareToken, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &token::UndeclareToken) -> Self::Output {
        // Header
        let header = declare::id::U_TOKEN;
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, x.id)?;

        Ok(())
    }
}

impl<R> RCodec<token::UndeclareToken, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<token::UndeclareToken, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);

        codec.read(reader)
    }
}

impl<R> RCodec<token::UndeclareToken, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<token::UndeclareToken, Self::Error> {
        if imsg::mid(self.header) != declare::id::U_TOKEN {
            return Err(DidntRead);
        }

        // Body
        let id: token::TokenId = self.codec.read(&mut *reader)?;

        // Extensions
        let has_ext = imsg::has_flag(self.header, token::flag::Z);
        if has_ext {
            extension::skip_all(reader, "ForgetToken")?;
        }

        Ok(token::UndeclareToken { id })
    }
}
