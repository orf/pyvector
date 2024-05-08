use std::collections::HashMap;
use std::sync::{RwLock};

use derivative::Derivative;
use serde_with::serde_as;
use vector::config::{
    ComponentKey, LogNamespace, SourceConfig, SourceOutput,
};
use vector::config::SourceContext;
use vector::event::{EventArray, EventContainer};
use vector::shutdown::ShutdownSignal;
use vector::SourceSender;
use vector_lib::buffers::topology::channel::{limited, LimitedReceiver, LimitedSender};
use vector_lib::codecs::{JsonDeserializer, JsonDeserializerConfig};
use vector_lib::codecs::decoding::DeserializerConfig;
use vector_lib::codecs::decoding::JsonDeserializerOptions;
use vector_lib::configurable::configurable_component;
use vector_lib::impl_generate_config_from_default;
use vector_lib::lookup::owned_value_path;
use vector_lib::Result as VectorResult;
use vector_lib::source::Source;
use vrl::value::Kind;

pub fn default_decoding() -> DeserializerConfig {
    JsonDeserializerConfig::new(JsonDeserializerOptions { lossy: false }).into()
}

/// Configuration for the python source
#[serde_as]
#[configurable_component(source(
    "python_source",
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
    senders: RwLock<HashMap<ComponentKey, LimitedSender<EventArray>>>,
}

impl ChannelRegistry {
    pub fn new_channel(&self, key: ComponentKey) -> LimitedReceiver<EventArray> {
        let (tx, rx) = limited(1000);
        let mut senders = self.senders.write().unwrap();
        senders.insert(key, tx);
        rx
    }

    pub fn get_sender(&self, key: &str) -> Option<LimitedSender<EventArray>> {
        let senders = self.senders.read().unwrap();
        senders.get(&ComponentKey::from(key)).cloned()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "python_source")]
impl SourceConfig for PythonSourceConfig {
    async fn build(&self, cx: SourceContext) -> VectorResult<Source> {
        let log_namespace = cx.log_namespace(None);
        let registry: &ChannelRegistry = cx.extra_context.get().unwrap();
        let rx = registry.new_channel(cx.key);
        Ok(Box::pin(python_source(
            rx,
            cx.shutdown,
            cx.out,
            log_namespace,
        )))
    }
    fn outputs(&self, global_log_namespace: LogNamespace) -> Vec<SourceOutput> {
        // let log_namespace = global_log_namespace.merge();
        let schema_definition = self
            .decoding
            .schema_definition(global_log_namespace)
            .with_standard_vector_source_metadata()
            .with_source_metadata(
                PythonSourceConfig::NAME,
                None,
                // Some(LegacyKey::InsertIfEmpty(owned_value_path!("service"))),
                &owned_value_path!("service"),
                Kind::bytes(),
                Some("service"),
            );

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
    mut stream: LimitedReceiver<EventArray>,
    mut shutdown: ShutdownSignal,
    mut out: SourceSender,
    log_namespace: LogNamespace,
) -> Result<(), ()> {
    println!("Python source started");
    // let mut stream = stream.borrow_mut();
    // if matches!(futures::poll!(&mut shutdown), Poll::Ready(_)) {
    //     break;
    // }
    let deserializer = JsonDeserializer::new(false);
    while let Some(mut events) = stream.next().await {
        // println!("Got item: {events:?}");
        // let events: Vec<_> = events.into_events().map(|mut event| {
        //     // event.
        //     let log = event.as_mut_log();
        //
        //     // println!("Got event: {:?}", log.get_message());
        //     // let now = chrono::Utc::now();
        //     // log_namespace.insert_standard_vector_source_metadata(
        //     //     log,
        //     //     PythonSourceConfig::NAME,
        //     //     now,
        //     // );
        //     // log_namespace.insert_source_metadata(
        //     //     PythonSourceConfig::NAME,
        //     //     log,
        //     //     Some(LegacyKey::InsertIfEmpty(path!("service"))),
        //     //     path!("service"),
        //     //     "vector",
        //     // );
        //
        //     event
        // }).collect();
        let iterator: Vec<_> = events.into_events().collect();
        out.send_batch(iterator).await.unwrap();
    }
    Ok(())
}
