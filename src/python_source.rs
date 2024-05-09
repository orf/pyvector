use bytes::Bytes;
use std::collections::HashMap;

use derivative::Derivative;
use serde_with::serde_as;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::RwLock;
use tracing::debug;
use vector::config::SourceContext;
use vector::config::{ComponentKey, LogNamespace, SourceConfig, SourceOutput};
use vector::shutdown::ShutdownSignal;
use vector::SourceSender;
use vector_lib::codecs::decoding::format::Deserializer as DeserializerTrait;
use vector_lib::codecs::decoding::JsonDeserializerOptions;
use vector_lib::codecs::decoding::{Deserializer, DeserializerConfig};
use vector_lib::codecs::{JsonDeserializerConfig};
use vector_lib::configurable::configurable_component;
use vector_lib::impl_generate_config_from_default;
use vector_lib::source::Source;
use vector_lib::Result as VectorResult;

pub fn default_decoding() -> DeserializerConfig {
    JsonDeserializerConfig::new(JsonDeserializerOptions { lossy: false }).into()
}

/// Configuration for the python source
#[serde_as]
#[configurable_component(source(
    "python",
    "Generate fake log events, which can be useful for testing and demos."
))]
#[derive(Clone, Debug, Derivative)]
#[derivative(Default)]
pub struct PythonSourceConfig {
    #[configurable(derived)]
    #[derivative(Default(value = "default_decoding()"))]
    #[serde(default = "default_decoding")]
    pub decoding: DeserializerConfig,
}

impl_generate_config_from_default!(PythonSourceConfig);

#[derive(Default, Debug)]
pub struct ChannelRegistry {
    senders: RwLock<HashMap<ComponentKey, Sender<Bytes>>>,
}

impl ChannelRegistry {
    pub async fn new_channel(&self, key: ComponentKey) -> Receiver<Bytes> {
        let (tx, rx) = channel(1024);
        let mut senders = self.senders.write().await;
        senders.insert(key, tx);
        rx
    }

    pub async fn get_sender(&self, key: &str) -> Option<Sender<Bytes>> {
        let senders = self.senders.read().await;
        senders.get(&ComponentKey::from(key)).cloned()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "python")]
impl SourceConfig for PythonSourceConfig {
    async fn build(&self, cx: SourceContext) -> VectorResult<Source> {
        let registry: &ChannelRegistry = cx.extra_context.get().unwrap();
        let log_namespace = cx.log_namespace(None);
        let reader = registry.new_channel(cx.key).await;
        let deserializer = self.decoding.build()?;

        Ok(Box::pin(python_source(
            reader,
            cx.shutdown,
            cx.out,
            log_namespace,
            deserializer,
        )))
    }
    fn outputs(&self, global_log_namespace: LogNamespace) -> Vec<SourceOutput> {
        let schema_definition = self.decoding.schema_definition(global_log_namespace);

        vec![SourceOutput::new_logs(
            self.decoding.output_type(),
            schema_definition,
        )]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn python_source(
    mut receiver: Receiver<Bytes>,
    mut shutdown: ShutdownSignal,
    mut out: SourceSender,
    log_namespace: LogNamespace,
    deserializer: Deserializer,
) -> Result<(), ()> {
    debug!("Python source started");
    // let mut stream = ReceiverStream::new(receiver);
    let mut counter = 0;

    let mut buffer: Vec<Bytes> = Vec::with_capacity(16);
    let mut events = Vec::with_capacity(16);

    loop {
        tokio::select! {
            biased;

            size = receiver.recv_many(&mut buffer, 16) => {
                if size == 0 {
                    break
                }
                for message in buffer.drain(0..size) {
                    let parsed = deserializer.parse(
                        message, log_namespace
                    ).unwrap();
                    events.extend(parsed);

                }

                out.send_batch(events.drain(0..size)).await.unwrap();
                counter += size;
            }

            _ = &mut shutdown => {
                debug!("Shutting down");
                break;
            },
        }
    }
    debug!("Python source processed {counter} messages");
    Ok(())
}
