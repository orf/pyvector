use crate::python_source::ChannelRegistry;
use std::num::NonZeroU64;
use std::process::ExitStatus;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use vector::app::{build_runtime, init_logging, Application, ApplicationConfig};
use vector::cli::{Color, LogFormat, RootOpts};
use vector::config::{load, Config, Format};
pub use vector::event::{EventArray, LogEvent};
pub use vector::extra_context::ExtraContext;
pub use vector::signal::{SignalPair, SignalTx, SignalTo};
pub use vector_lib::metrics::Controller;

//pub mod sources;
pub mod python_source;

pub fn create_config(contents: &str) -> Config {
    let mut builder = Config::builder();
    builder
        .append(load(contents.as_bytes(), Format::Toml).unwrap())
        .unwrap();
    let config = builder.build().unwrap();
    config
}

pub fn create_app(config: Config) -> (JoinHandle<ExitStatus>, Runtime, ExtraContext, SignalTx, &'static Controller) {
    let opts = RootOpts {
        config_paths: vec![],
        config_dirs: vec![],
        config_paths_toml: vec![],
        config_paths_json: vec![],
        config_paths_yaml: vec![],
        require_healthy: Some(true),
        threads: None,
        verbose: 0,
        quiet: 0,
        log_format: LogFormat::Text,
        color: Color::Auto,
        watch_config: false,
        internal_log_rate_limit: 0,
        graceful_shutdown_limit_secs: NonZeroU64::new(60).unwrap(),

        no_graceful_shutdown_limit: false,
        allocation_tracing: true,
        allocation_tracing_reporting_interval_ms: 10,
        openssl_no_probe: false,
        allow_empty_config: false,
    };

    opts.init_global();
    let color = opts.color.use_color();
    init_logging(color, opts.log_format, "debug", opts.internal_log_rate_limit);

    let runtime = build_runtime(opts.threads, "vector-worker").unwrap();
    let mut signals = SignalPair::new(&runtime);

    let context = ExtraContext::single_value(ChannelRegistry::default());

    let config = runtime
        .block_on(ApplicationConfig::from_config(
            vec![],
            config,
            context.clone(),
        ))
        .unwrap();

    let tx = signals.handler.clone_tx();

    let app = Application {
        root_opts: opts,
        config,
        signals,
    };
    let started_app = app.start(runtime.handle()).unwrap();
    let handle = runtime.spawn(started_app.run());
    let controller = Controller::get().unwrap();

    (handle, runtime, context, tx, controller)
}
