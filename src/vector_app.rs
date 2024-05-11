use crate::python_source::ChannelRegistry;
use crate::vector_context::VectorContext;
use bytes::Bytes;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Notify};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tracing::{debug, warn};
use vector::config::Config;
use vector::extra_context::ExtraContext;
use vector::signal::{ShutdownError, SignalRx, SignalTo};
use vector::topology::{
    RunningTopology, SharedTopologyController, TopologyController,
};

pub enum VectorApp {
    Pending {
        context: ExtraContext,
        config: Config,
        signal: SignalRx,
        runtime: Arc<Runtime>,
    },
    Running {
        runtime: Arc<Runtime>,
        task: JoinHandle<()>,
        shutdown_notifier: Arc<Notify>,
        context: ExtraContext,
    },
    Stopped,
}

impl VectorApp {
    pub fn new(config: Config, vector_context: &VectorContext) -> Self {
        let context = ExtraContext::single_value(ChannelRegistry::default());
        let signal = vector_context.new_subscription();
        let runtime = vector_context.runtime.clone();
        Self::Pending {
            context,
            config,
            signal,
            runtime,
        }
    }
}

impl VectorApp {
    pub async fn start(self) -> VectorApp {
        if let VectorApp::Pending {
            context,
            config,
            signal,
            runtime,
        } = self
        {
            let (topology_controller, shutdown_receiver) = runtime
                .spawn(Self::start_running(config, context.clone()))
                .await
                .unwrap();
            let shutdown_notifier = Arc::new(Notify::new());
            let shutdown_notified = shutdown_notifier.clone();
            let task = runtime.spawn(Self::pump_signal_events(
                topology_controller,
                signal,
                shutdown_receiver,
                shutdown_notified,
            ));
            return VectorApp::Running {
                task,
                runtime,
                shutdown_notifier,
                context,
            };
        };
        todo!("start error handling")
    }

    pub async fn stop(self) -> VectorApp {
        if let VectorApp::Running {
            task,
            shutdown_notifier,
            ..
        } = self
        {
            shutdown_notifier.notify_one();
            task.await.unwrap();
            return VectorApp::Stopped;
        };
        todo!("stop error handling")
    }

    pub async fn get_sender(&self, name: &str) -> Sender<Bytes> {
        if let VectorApp::Running { context, .. } = self {
            let reg: &ChannelRegistry = context.get().unwrap();
            return reg.get_sender(name).await.unwrap();
        }
        todo!("get sender error handling")
    }

    async fn start_running(
        config: Config,
        context: ExtraContext,
    ) -> (
        SharedTopologyController,
        UnboundedReceiverStream<ShutdownError>,
    ) {
        let (topology, graceful_crash_receiver) =
            RunningTopology::start_init_validated(config, context.clone())
                .await
                .unwrap();

        let topology_controller = SharedTopologyController::new(TopologyController {
            topology,
            config_paths: vec![],
            require_healthy: Some(true),
            extra_context: context.clone(),
        });
        let graceful_crash = UnboundedReceiverStream::new(graceful_crash_receiver);

        (topology_controller, graceful_crash)
    }

    async fn pump_signal_events(
        topology_controller: SharedTopologyController,
        mut signal_rx: SignalRx,
        mut graceful_crash: UnboundedReceiverStream<ShutdownError>,
        shutdown_notify: Arc<Notify>,
    ) {
        let signal = loop {
            let has_sources = !topology_controller
                .lock()
                .await
                .topology
                .config()
                .is_empty();
            tokio::select! {
                _ = shutdown_notify.notified() => {
                    break SignalTo::Shutdown(None)
                }
                signal = signal_rx.recv() => {
                    match signal {
                        Err(RecvError::Lagged(amt)) => {
                            warn!("Overflow, dropped {} signals.", amt);
                        }
                        Err(RecvError::Closed) => break SignalTo::Shutdown(None),
                        Ok(SignalTo::Quit) => break SignalTo::Quit,
                        Ok(SignalTo::Shutdown(e)) => break SignalTo::Shutdown(e),
                        _ => {}
                    }
                }
                // Trigger graceful shutdown if a component crashed, or all sources have ended.
                error = graceful_crash.next() => break SignalTo::Shutdown(error),
                _ = TopologyController::sources_finished(topology_controller.clone()), if has_sources => {
                    debug!("All sources have finished.");
                    break SignalTo::Shutdown(None)
                } ,
                else => unreachable!("Signal streams never end"),
            }
        };

        let topology_controller = topology_controller
            .try_into_inner()
            .expect("fail to unwrap topology controller")
            .into_inner();

        match signal {
            SignalTo::Shutdown(_) => {
                tokio::select! {
                    _ = topology_controller.stop() => {
                    }, // Graceful shutdown finished
                    _ = signal_rx.recv() => {
                    },
                }
            }
            SignalTo::Quit => {
            }
            _ => unreachable!(),
        }
    }
}
