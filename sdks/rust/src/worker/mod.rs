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

pub(super) mod data;
mod external_worker_service;
pub mod operators;

mod coder_from_urn;
pub(crate) use coder_from_urn::coder_from_urn;
pub use coder_from_urn::{CustomCoderFromUrn, CUSTOM_CODER_FROM_URN};

mod interceptors;

pub use external_worker_service::ExternalWorkerPool;
pub use operators::Receiver;

pub mod sdk_worker;
pub mod worker_main;

// TODO: organize this in a better way
pub mod test_utils {
    use std::sync::Mutex;

    pub static RECORDING_OPERATOR_LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub fn reset_log() {
        let mut log = RECORDING_OPERATOR_LOGS.lock().unwrap();
        *log.as_mut() = Vec::new();
    }
}

#[cfg(test)]
mod data_test;
#[cfg(test)]
mod worker_test;
