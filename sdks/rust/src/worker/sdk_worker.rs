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

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};

use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;
use tonic::Status;

use crate::proto::{
    fn_execution_v1, fn_execution_v1::beam_fn_control_client as beam_fn_control_client_v1,
    fn_execution_v1::instruction_request as instruction_request_v1,
    fn_execution_v1::instruction_response as instruction_response_v1, pipeline_v1,
};

use crate::worker::data::MultiplexingDataChannel;
use crate::worker::interceptors::WorkerIdInterceptor;
use crate::worker::operators::{create_operator, Operator, OperatorContext, OperatorI, Receiver};

type BundleDescriptorId = String;
type InstructionId = String;

// TODO(sjvanrossum): Convert simple map caches to concurrent caches.
// Using concurrent caches removes the need to synchronize on the worker instance in every context.
#[derive(Debug)]
pub struct Worker {
    _id: String,
    // Cheap and safe to clone
    control_client: beam_fn_control_client_v1::BeamFnControlClient<
        InterceptedService<Channel, WorkerIdInterceptor>,
    >,
    // Cheap and safe to clone
    control_tx: mpsc::Sender<fn_execution_v1::InstructionResponse>,
    control_rx: Arc<TokioMutex<mpsc::Receiver<fn_execution_v1::InstructionResponse>>>,
    // Cheap and safe to clone
    process_bundle_descriptors:
        moka::future::Cache<BundleDescriptorId, Arc<fn_execution_v1::ProcessBundleDescriptor>>,
    _bundle_processors: DashMap<String, BundleProcessor>,
    _active_bundle_processors: DashMap<String, BundleProcessor>,
    _data_channels: DashMap<String, MultiplexingDataChannel>,
    _options: HashMap<String, String>,
}

impl Worker {
    // concurrent data structures and/or finer grained locks.
    pub fn try_new(
        id: String,
        control_endpoint: String,
        _logging_endpoint: String,
        _status_endpoint: Option<String>,
        _options: serde_json::Value,
        _runner_capabilities: HashSet<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let control_client = beam_fn_control_client_v1::BeamFnControlClient::with_interceptor(
            Channel::from_shared(control_endpoint)?.connect_lazy(),
            WorkerIdInterceptor::new(id.clone()),
        );
        let (control_tx, control_rx) = mpsc::channel(100);

        Ok(Self {
            _id: id,
            control_client,
            control_tx,
            control_rx: Arc::new(TokioMutex::new(control_rx)),
            // TODO(sjvanrossum): Maybe define the eviction policy
            process_bundle_descriptors: moka::future::Cache::builder().build(),
            _bundle_processors: DashMap::new(),
            _active_bundle_processors: DashMap::new(),
            _data_channels: DashMap::new(),
            _options: HashMap::new(),
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let rx = self.control_rx.clone();
        let outbound = async_stream::stream! {
            while let Some(control_res) = rx.lock().await.recv().await {
                yield control_res
            }
        };

        let response = self.control_client.control(outbound).await?;
        let mut inbound = response.into_inner();

        while let Some(control_req) = inbound.message().await? {
            match control_req.request {
                Some(instruction_request_v1::Request::ProcessBundle(instr_req)) => {
                    self.process_bundle(control_req.instruction_id, instr_req);
                }
                Some(instruction_request_v1::Request::ProcessBundleProgress(instr_req)) => {
                    self.process_bundle_progress(control_req.instruction_id, instr_req);
                }
                Some(instruction_request_v1::Request::ProcessBundleSplit(instr_req)) => {
                    self.process_bundle_split(control_req.instruction_id, instr_req);
                }
                Some(instruction_request_v1::Request::FinalizeBundle(instr_req)) => {
                    self.finalize_bundle(control_req.instruction_id, instr_req);
                }
                Some(instruction_request_v1::Request::MonitoringInfos(instr_req)) => {
                    self.monitoring_infos(control_req.instruction_id, instr_req);
                }
                Some(instruction_request_v1::Request::HarnessMonitoringInfos(instr_req)) => {
                    self.harness_monitoring_infos(control_req.instruction_id, instr_req);
                }
                Some(instruction_request_v1::Request::Register(instr_req)) => {
                    self.register(control_req.instruction_id, instr_req);
                }
                _ => {
                    self.fail(
                        control_req.instruction_id.clone(),
                        format!("Unexpected request: {:?}", control_req),
                        self.control_tx.clone(),
                    )
                    .await?;
                }
            };
        }
        Ok(())
    }

    pub async fn stop(&self) {
        self.control_rx.lock().await.close()
    }

    fn process_bundle(
        &self,
        _instruction_id: InstructionId,
        request: fn_execution_v1::ProcessBundleRequest,
    ) {
        let mut client = self.control_client.clone();
        let descriptor_cache = self.process_bundle_descriptors.clone();
        tokio::spawn(async move {
            let _descriptor = descriptor_cache
                .try_get_with::<_, Status>(request.process_bundle_descriptor_id.clone(), async {
                    let res = client
                        .get_process_bundle_descriptor(
                            fn_execution_v1::GetProcessBundleDescriptorRequest {
                                process_bundle_descriptor_id: request
                                    .process_bundle_descriptor_id
                                    .clone(),
                            },
                        )
                        .await?;
                    Ok(Arc::new(res.into_inner()))
                })
                .await
                .unwrap();
            // TODO(sjvanrossum): Fetch bundle processor and process bundle
        });
    }

    fn process_bundle_progress(
        &self,
        _instruction_id: InstructionId,
        _request: fn_execution_v1::ProcessBundleProgressRequest,
    ) {
        // TODO(sjvanrossum): Flesh out after process_bundle is sufficiently implemented
    }

    fn process_bundle_split(
        &self,
        _instruction_id: InstructionId,
        _request: fn_execution_v1::ProcessBundleSplitRequest,
    ) {
        // TODO(sjvanrossum): Flesh out after process_bundle is sufficiently implemented
    }

    fn finalize_bundle(
        &self,
        _instruction_id: InstructionId,
        _request: fn_execution_v1::FinalizeBundleRequest,
    ) {
        // TODO(sjvanrossum): Flesh out after process_bundle is sufficiently implemented.
    }

    fn monitoring_infos(
        &self,
        _instruction_id: InstructionId,
        _request: fn_execution_v1::MonitoringInfosMetadataRequest,
    ) {
        // TODO: Implement
    }

    fn harness_monitoring_infos(
        &self,
        _instruction_id: InstructionId,
        _request: fn_execution_v1::HarnessMonitoringInfosRequest,
    ) {
        // TODO: Implement
    }

    fn register(&self, instruction_id: InstructionId, request: fn_execution_v1::RegisterRequest) {
        let descriptor_cache = self.process_bundle_descriptors.clone();
        let tx = self.control_tx.clone();
        tokio::spawn(async move {
            for descriptor in request.process_bundle_descriptor {
                descriptor_cache
                    .insert(descriptor.id.clone(), Arc::new(descriptor))
                    .await;
            }

            tx.send(fn_execution_v1::InstructionResponse {
                instruction_id,
                error: String::default(),
                response: Some(instruction_response_v1::Response::Register(
                    fn_execution_v1::RegisterResponse::default(),
                )),
            })
            .await
            .unwrap()
        });
    }

    async fn fail(
        &self,
        instruction_id: InstructionId,
        error: String,
        tx: mpsc::Sender<fn_execution_v1::InstructionResponse>,
    ) -> Result<(), mpsc::error::SendError<fn_execution_v1::InstructionResponse>> {
        tx.send(fn_execution_v1::InstructionResponse {
            instruction_id,
            error,
            response: None,
        })
        .await
    }
}

#[derive(Debug)]
pub struct BundleProcessor {
    descriptor: Arc<fn_execution_v1::ProcessBundleDescriptor>,
    creation_ordered_operators: Mutex<Vec<Arc<Operator>>>,
    topologically_ordered_operators: RwLock<Vec<Arc<Operator>>>,

    operators: RwLock<HashMap<String, Arc<Operator>>>,
    receivers: RwLock<HashMap<String, Arc<Receiver>>>,
    consumers: RwLock<HashMap<String, Vec<String>>>,

    current_bundle_id: Mutex<Option<String>>,
}

impl BundleProcessor {
    pub fn new(
        descriptor: Arc<fn_execution_v1::ProcessBundleDescriptor>,
        root_urns: &[&'static str],
    ) -> Arc<Self> {
        let mut consumers: HashMap<String, Vec<String>> = HashMap::new();
        for (transform_id, ptransform) in descriptor.transforms.iter() {
            if is_primitive(ptransform) {
                ptransform
                    .inputs
                    .values()
                    .for_each(|pcollection_id: &String| {
                        if consumers.get(pcollection_id).is_none() {
                            consumers.insert(pcollection_id.clone(), Vec::new());
                        }

                        consumers
                            .get_mut(pcollection_id)
                            .unwrap()
                            .push(transform_id.clone());
                    })
            }
        }

        let mut _instance = Arc::new(Self {
            descriptor: descriptor.clone(),
            creation_ordered_operators: Mutex::new(Vec::new()),
            topologically_ordered_operators: RwLock::new(Vec::new()),
            operators: RwLock::new(HashMap::new()),
            receivers: RwLock::new(HashMap::new()),
            consumers: RwLock::new(consumers),
            current_bundle_id: Mutex::new(None),
        });

        descriptor
            .transforms
            .iter()
            .for_each(|(transform_id, transform)| {
                if let Some(spec) = &transform.spec {
                    if root_urns.contains(&spec.urn.as_str()) {
                        BundleProcessor::get_operator(_instance.clone(), transform_id.clone());
                    }
                }
            });

        _instance.create_topologically_ordered_operators();

        _instance
    }

    // TODO: the bundle processors are being be passed around by value as parameters
    // to avoid closures with temp references in an async context. However, this
    // should be revisited later.
    fn get_receiver(
        bundle_processor: Arc<BundleProcessor>,
        pcollection_id: String,
    ) -> Arc<Receiver> {
        let receivers = bundle_processor.receivers.read().unwrap();

        let receiver = match receivers.get(&pcollection_id) {
            Some(rec) => rec.clone(),
            None => {
                let consumers = bundle_processor.consumers.read().unwrap();

                let pcoll_operators: Vec<Arc<Operator>> = match consumers.get(&pcollection_id) {
                    Some(v) => {
                        drop(receivers);

                        let mut oprs = Vec::new();
                        for transform_id in v {
                            let cl = bundle_processor.clone();
                            oprs.push(BundleProcessor::get_operator(cl, transform_id.clone()))
                        }
                        oprs
                    }
                    None => {
                        drop(receivers);
                        Vec::new()
                    }
                };

                let receiver = Receiver::new(pcoll_operators);

                let mut receivers = bundle_processor.receivers.write().unwrap();
                receivers.insert(pcollection_id.clone(), Arc::new(receiver));
                receivers[&pcollection_id].clone()
            }
        };

        receiver
    }

    fn get_operator(bundle_processor: Arc<BundleProcessor>, transform_id: String) -> Arc<Operator> {
        let get_receiver = |b: Arc<BundleProcessor>, pcollection_id: String| {
            BundleProcessor::get_receiver(b, pcollection_id)
        };

        let operators = bundle_processor.operators.read().unwrap();

        let operator_opt = operators.get(&transform_id);

        if let Some(operator) = operator_opt {
            operator.clone()
        } else {
            let descriptor = bundle_processor.descriptor.clone();
            drop(operators);

            let op = create_operator(
                &transform_id,
                Arc::new(OperatorContext {
                    descriptor,
                    get_receiver: Box::new(get_receiver),

                    bundle_processor: bundle_processor.clone(),
                }),
            );

            let mut operators = bundle_processor.operators.write().unwrap();
            operators.insert(transform_id.clone(), Arc::new(op));

            let mut creation_ordered_operators =
                bundle_processor.creation_ordered_operators.lock().unwrap();
            creation_ordered_operators.push(operators[&transform_id].clone());

            operators[&transform_id].clone()
        }
    }

    fn create_topologically_ordered_operators(&self) {
        let creation_ordered = self.creation_ordered_operators.lock().unwrap();

        let mut instance_operators = self.topologically_ordered_operators.write().unwrap();

        creation_ordered
            .iter()
            .rev()
            .for_each(|op: &Arc<Operator>| {
                instance_operators.push(op.clone());
            });
    }

    pub async fn process(&self, instruction_id: String) {
        let mut current_bundle_id = self.current_bundle_id.lock().unwrap();
        current_bundle_id.replace(instruction_id);
        drop(current_bundle_id);

        let topologically_ordered_operators = self.topologically_ordered_operators.read().unwrap();
        for o in topologically_ordered_operators.iter().rev() {
            o.start_bundle();
        }

        for o in topologically_ordered_operators.iter() {
            o.finish_bundle();
        }
    }
}

// TODO: Handle transforms that return their inputs.
fn is_primitive(transform: &pipeline_v1::PTransform) -> bool {
    transform.subtransforms.is_empty()
}
