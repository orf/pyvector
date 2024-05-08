use lib::python_source::ChannelRegistry;
use lib::{Controller, create_app, create_config, EventArray, ExtraContext, LogEvent, SignalTo, SignalTx};
use pyo3::prelude::*;
use serde_json::Value;
use std::process::ExitStatus;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[pyclass(frozen)]
struct Vector {
    app: Mutex<Option<JoinHandle<ExitStatus>>>,
    // #[pyo3(get)]
    // config: PathBuf,

    // app: Mutex<Option<VectorApp>>,
    runtime: Runtime,
    context: ExtraContext,
    tx: SignalTx,
    metrics: &'static Controller,
}

#[pymethods]
impl Vector {
    #[new]
    fn new(config: &str) -> Self {
        let config = create_config(config);
        let (app, runtime, context, tx, metrics) = create_app(config);
        Self {
            runtime,
            app: Mutex::new(Some(app)),
            context,
            tx,
            metrics
        }
    }

    fn get_metrics(&self) {
        let metrics = self.metrics.capture_metrics();
        for metric in metrics {
            println!("Name: {}, Value: {}", metric.name(), metric.value());
        }
    }

    async fn stop(&self) {
        let shutdown = SignalTo::Shutdown(None);
        self.tx.send(shutdown).unwrap();
        let mut app = self.app.lock().await;
        let taken = app.take().unwrap();
        taken.await.unwrap();
    }

    async fn send_batch(&self, name: String, contents: Vec<Vec<u8>>) {
        let reg: &ChannelRegistry = self.context.get().unwrap();
        let mut sender = reg.get_sender(&name).unwrap();
        let log_array: Vec<_> = contents
            .into_iter()
            .map(|c| {
                let value: Value = serde_json::from_slice(&c).unwrap();
                LogEvent::try_from(value).unwrap()
            })
            .collect();
        let event_array = EventArray::Logs(log_array);
        sender.send(event_array).await.unwrap();
    }

    async fn send(&self, name: String, contents: Vec<u8>) {
        self.send_batch(name, vec![contents]).await;
    }
}

#[pymodule]
fn pyvector(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Vector>()?;
    Ok(())
}
