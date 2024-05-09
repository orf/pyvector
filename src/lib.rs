mod python_source;
mod vector_app;
mod vector_context;

use crate::vector_app::VectorApp;
use crate::vector_context::VectorContext;
use bytes::Bytes;
use pyo3::prelude::*;
use tokio::sync::{RwLock};
use vector::config::{load, Config, Format};

pub fn create_config(contents: &str) -> Config {
    let mut builder = Config::builder();
    builder
        .append(load(contents.as_bytes(), Format::Toml).unwrap())
        .unwrap();

    builder.build().unwrap()
}

#[pyclass(frozen)]
struct Vector {
    app: RwLock<Option<VectorApp>>, // app: Mutex<Option<JoinHandle<ExitStatus>>>,
                                    // runtime: Runtime,
                                    // context: ExtraContext,
                                    // tx: SignalTx,
                                    // metrics: &'static Controller,
}

#[pymethods]
impl Vector {
    #[new]
    fn new(config: &str) -> Self {
        let config = create_config(config);
        let context = VectorContext::global();
        let app = VectorApp::new(config, context);
        Self {
            app: RwLock::new(Some(app)),
        }
    }

    async fn start(&self) {
        let mut app_lock = self.app.write().await;
        let app = app_lock.take().unwrap();
        let started = app.start().await;
        app_lock.replace(started);
    }

    async fn stop(&self) {
        let mut app_lock = self.app.write().await;
        let app = app_lock.take().unwrap();
        let stopped = app.stop().await;
        app_lock.replace(stopped);
    }

    async fn send(&self, source: String, data: Vec<u8>) {
        let app_lock = self.app.read().await;
        if let Some(app) = app_lock.as_ref() {
            let sender = app.get_sender(&source).await;
            sender.send(Bytes::from(data)).await.unwrap();
        }
    }

    // fn get_metrics(&self) {
    //     let metrics = self.metrics.capture_metrics();
    //     for metric in metrics {
    //         println!("Name: {}, Value: {}", metric.name(), metric.value());
    //     }
    // }
}

#[pymodule]
fn pyvector(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Vector>()?;
    Ok(())
}
