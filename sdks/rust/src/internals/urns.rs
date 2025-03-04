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

pub const IMPULSE_BUFFER: &[u8] = "impulse".as_bytes();

pub const DATA_INPUT_URN: &str = "beam:runner:source:v1";
pub const DATA_OUTPUT_URN: &str = "beam:runner:sink:v1";
pub const IMPULSE_URN: &str = "beam:transform:impulse:v1";
pub const PAR_DO_URN: &str = "beam:beam:pardo:v1";
pub const GROUP_BY_KEY_URN: &str = "beam:beam:group_by_key:v1";
pub const FLATTEN_URN: &str = "beam:beam:flatten:v1";
pub const IDENTITY_DOFN_URN: &str = "beam:dofn:identity:0.1";

// TODO: move test urns elsewhere
pub const CREATE_URN: &str = "create";
pub const RECORDING_URN: &str = "recording";
pub const PARTITION_URN: &str = "partition";
