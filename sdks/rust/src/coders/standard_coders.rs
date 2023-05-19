/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! These are the coders necessary for encoding the data types required by
//! the Apache Beam model. They provide standardized ways of encode data for
//! communication between the runner, the Beam workers, and the user's code.
//! For example for any aggregations the runner and the SDK need to agree on
//! the encoding of key-value pairs; so that the SDK will encode keys properly,
//! and the runner will be able to group elements of the
//! same key together.
//!
//! The formal specifications for these coders can be found in
//! model/pipeline/src/main/proto/beam_runner_api.proto

use std::fmt;
use std::io::{self, ErrorKind, Read, Write};

use bytes::Bytes;
use integer_encoding::{VarInt, VarIntReader, VarIntWriter};

use crate::coders::required_coders::BytesCoder;
use crate::coders::{urns::*, CoderUrn};
use crate::coders::{Coder, Context};
use crate::elem_types::ElemType;

#[derive(Clone, Default)]
pub struct StrUtf8Coder {}

impl CoderUrn for StrUtf8Coder {
    const URN: &'static str = STR_UTF8_CODER_URN;
}

// TODO: accept string references as well?
impl Coder for StrUtf8Coder {
    fn encode(
        &self,
        element: &dyn ElemType,
        writer: &mut dyn Write,
        context: &Context,
    ) -> Result<usize, io::Error> {
        let element = element.as_any().downcast_ref::<String>().unwrap();

        let bytes = Bytes::from(element.as_bytes().to_vec());
        let coder = BytesCoder::default();
        coder.encode(&bytes, writer, context)
    }

    fn decode(
        &self,
        reader: &mut dyn Read,
        context: &Context,
    ) -> Result<Box<dyn ElemType>, io::Error> {
        let coder = BytesCoder::default();
        let bytes = coder.decode(reader, context)?;
        let bytes = bytes.as_any().downcast_ref::<Bytes>().unwrap();

        let res = String::from_utf8(bytes.as_ref().to_vec());

        //TODO: improve error handling
        match res {
            Ok(s) => Ok(Box::new(s)),
            Err(_) => Result::Err(io::Error::new(
                ErrorKind::Other,
                "Unable to convert bytes to string",
            )),
        }
    }

    fn component_coder_urns() -> Vec<super::CoderUrnTree> {
        vec![]
    }
}

impl fmt::Debug for StrUtf8Coder {
    fn fmt(&self, o: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        o.debug_struct("StrUtf8Coder")
            .field("urn", &Self::URN)
            .finish()
    }
}

#[derive(Clone)]
pub struct VarIntCoder<N: fmt::Debug + VarInt> {
    _var_int_type: std::marker::PhantomData<N>,
}

impl<N> CoderUrn for VarIntCoder<N>
where
    N: fmt::Debug + VarInt + ElemType,
{
    const URN: &'static str = VARINT_CODER_URN;
}

// TODO: passes tests for -1 if it gets casted to u64 and encoded as such.
// Revisit this later
impl<N> Coder for VarIntCoder<N>
where
    N: fmt::Debug + VarInt + ElemType,
{
    // TODO: try to adapt Coder such that the context arg is not mandatory
    fn encode(
        &self,
        element: &dyn ElemType,
        mut writer: &mut dyn Write,
        _context: &Context,
    ) -> Result<usize, io::Error> {
        let element = element.as_any().downcast_ref::<N>().unwrap();

        writer.write_varint(*element)
    }

    fn decode(
        &self,
        mut reader: &mut dyn Read,
        _context: &Context,
    ) -> Result<Box<dyn ElemType>, io::Error> {
        let element: N = reader.read_varint()?;
        Ok(Box::new(element))
    }

    fn component_coder_urns() -> Vec<super::CoderUrnTree> {
        vec![]
    }
}

impl<N: fmt::Debug + VarInt> Default for VarIntCoder<N> {
    fn default() -> Self {
        Self {
            _var_int_type: std::marker::PhantomData,
        }
    }
}

impl<N: fmt::Debug + VarInt + ElemType> fmt::Debug for VarIntCoder<N> {
    fn fmt(&self, o: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        o.debug_struct("VarIntCoder")
            .field("urn", &Self::URN)
            .finish()
    }
}

pub struct NullableCoder<E: ElemType> {
    _e: std::marker::PhantomData<E>,
}

impl<E> CoderUrn for NullableCoder<E>
where
    E: ElemType,
{
    const URN: &'static str = NULLABLE_CODER_URN;
}

impl<E> Coder for NullableCoder<E>
where
    E: ElemType,
{
    fn encode(
        &self,
        _element: &dyn ElemType,
        _writer: &mut dyn Write,
        _context: &Context,
    ) -> Result<usize, io::Error> {
        todo!()
    }

    fn decode(
        &self,
        _reader: &mut dyn Read,
        _context: &Context,
    ) -> Result<Box<dyn ElemType>, io::Error> {
        todo!()
    }

    fn component_coder_urns() -> Vec<super::CoderUrnTree> {
        vec![]
    }
}

impl<E> fmt::Debug for NullableCoder<E>
where
    E: ElemType,
{
    fn fmt(&self, o: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        o.debug_struct("NullableCoder")
            .field("urn", &Self::URN)
            .finish()
    }
}

impl<E> Default for NullableCoder<E>
where
    E: ElemType,
{
    fn default() -> Self {
        Self {
            _e: std::marker::PhantomData,
        }
    }
}
