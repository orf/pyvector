use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::runtime::Runtime;
use vector::app::{build_runtime, init_logging, ApplicationConfig};
use vector::cli::LogFormat;
use vector::config::Config;
use vector::extra_context::ExtraContext;
use vector::heartbeat;
use vector::signal::{SignalPair, SignalRx};

pub struct VectorContext {
    pub runtime: Arc<Runtime>,
    signals: SignalPair,
}

impl VectorContext {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceCell<VectorContext> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            openssl_probe::init_ssl_cert_env_vars();

            vector::metrics::init_global().expect("metrics initialization failed");
            init_logging(false, LogFormat::Text, "debug", 10);
            let runtime = build_runtime(None, "vector-worker").unwrap();
            let signals = SignalPair::new(&runtime);
            runtime.spawn(heartbeat::heartbeat());

            Self {
                runtime: Arc::new(runtime),
                signals,
            }
        })
    }

    pub fn new_subscription(&self) -> SignalRx {
        self.signals.handler.subscribe()
    }

    pub async fn create_application_config(
        &self,
        config: Config,
        context: ExtraContext,
    ) -> ApplicationConfig {
        self.runtime.spawn(async move {
            ApplicationConfig::from_config(
                vec![],
                config,
                context.clone(),
            ).await
        }).await.unwrap().unwrap()
    }
}
