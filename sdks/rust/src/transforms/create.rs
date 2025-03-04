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

use super::impulse::Impulse;
use super::pardo::ParDo;
use crate::{
    elem_types::{DefaultCoder, ElemType},
    internals::pvalue::{PTransform, PValue},
};

pub struct Create<Out> {
    elements: Vec<Out>,
}

impl<Out: ElemType> Create<Out> {
    pub fn new(elements: Vec<Out>) -> Self {
        Self { elements }
    }
}

impl<Out: ElemType + DefaultCoder + Clone> PTransform<(), Out> for Create<Out> {
    fn expand_internal(
        &self,
        input: &PValue<()>,
        _pipeline: std::sync::Arc<crate::internals::pipeline::Pipeline>,
        _out_coder_urn: &crate::coders::CoderUrnTree,
        _transform_proto: &mut crate::proto::pipeline_v1::PTransform,
    ) -> PValue<Out>
    where
        Self: Sized,
    {
        let elements = self.elements.to_vec();
        // TODO: Consider reshuffling.
        input
            .apply(Impulse::new())
            .apply(ParDo::from_flat_map(move |_x| -> Vec<Out> {
                elements.to_vec()
            }))
    }
}
