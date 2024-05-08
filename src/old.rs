pub mod source;
pub use source::PythonSourceConfig;

use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::Mutex;
use pyo3::{prelude::*};
use pyo3::exceptions::PyValueError;
use tokio::runtime::Runtime;
use vector::app::{Application};
use vector::cli::{Color, LogFormat, Opts, RootOpts};
use tokio::task::JoinHandle;

struct VectorApp {
    runtime: Runtime,
    handle: JoinHandle<ExitStatus>,
}

#[pyclass(frozen)]
struct Vector {
    #[pyo3(get)]
    config: PathBuf,

    app: Mutex<Option<VectorApp>>,
}

#[pymethods]
impl Vector {
    #[new]
    fn new(config: PathBuf) -> Self {
        Self {
            config,
            app: Mutex::new(None),
        }
    }

    fn is_running(&self) -> bool {
        let mut locked = self.app.lock().unwrap();
        match locked.deref() {
            None => {
                false
            }
            Some(app) => {
                !app.handle.is_finished()
            }
        }
    }

    fn stop(&self) -> PyResult<()> {
        let mut locked = self.app.lock().unwrap();
        match locked.deref() {
            None => {
                return Err(PyValueError::new_err("Not started"));
            }
            Some(app) => {
                app.handle.abort();
                Ok(())
            }
        }

    }

    fn start(&self) -> PyResult<()> {
        let mut locked = self.app.lock().unwrap();
        let app = locked.deref_mut();
        match app {
            None => {
                let (runtime, application) = Application::prepare_from_opts(
                    create_opts(self.config.clone()),
                    Default::default(),
                ).unwrap();
                let started_app = application.start(runtime.handle()).unwrap();
                let handle = runtime.spawn(started_app.run());
                let vector_app = VectorApp {
                    runtime,
                    handle
                };
                app.replace(vector_app);
            }
            Some(_) => {
                return Err(PyValueError::new_err("Already started"));
            }
        }
        Ok(())
    }
}

fn create_opts(config: PathBuf) -> Opts {
    Opts {
        root: RootOpts {
            config_paths: vec![],
            config_dirs: vec![],
            config_paths_toml: vec![
                config
            ],
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
            // allocation_tracing: false,
            // allocation_tracing_reporting_interval_ms: 0,
            openssl_no_probe: false,
            allow_empty_config: false,
        },
        sub_command: None,
    }
}

// #[pyfunction]
// fn start_vector(config: PathBuf, result: Option<PyObject>) -> Option<PyObject> {
//     // let (runtime, app) = Application::prepare_from_opts(
//     //     Opts {
//     //         root: RootOpts {
//     //             config_paths: vec![],
//     //             config_dirs: vec![],
//     //             config_paths_toml: vec![
//     //                 config
//     //             ],
//     //             config_paths_json: vec![],
//     //             config_paths_yaml: vec![],
//     //             require_healthy: Some(true),
//     //             threads: None,
//     //             verbose: 0,
//     //             quiet: 0,
//     //             log_format: LogFormat::Text,
//     //             color: Color::Auto,
//     //             watch_config: false,
//     //             internal_log_rate_limit: 0,
//     //             graceful_shutdown_limit_secs: NonZeroU64::new(60).unwrap(),
//     //
//     //             no_graceful_shutdown_limit: false,
//     //             // allocation_tracing: false,
//     //             // allocation_tracing_reporting_interval_ms: 0,
//     //             openssl_no_probe: false,
//     //             allow_empty_config: false,
//     //         },
//     //         sub_command: None,
//     //     },
//     //     Default::default(),
//     // ).unwrap();
//
//     // std::thread::sleep(Duration::from_secs(1));
//     // result
// }

#[pymodule]
fn pyvector(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Vector>()?;
    Ok(())
}
//
//
// #[derive(Error, Debug)]
// pub enum AppError {
//     #[error("Exit code {code}")]
//     ExitCode { code: ExitCode },
//     #[error("General Error {source}")]
//     General {
//         #[from]
//         source: anyhow::Error,
//     },
// }

// async fn start_app(
//     config: PathBuf,
//     runtime: &Runtime,
//     mut signals: SignalPair,
// ) -> Result<(), AppError> {
//     let root = RootOpts {
//         config_paths: vec![],
//         config_dirs: vec![],
//         config_paths_toml: vec![config],
//         config_paths_json: vec![],
//         config_paths_yaml: vec![],
//         require_healthy: Some(true),
//         threads: None,
//         verbose: 0,
//         quiet: 0,
//         log_format: LogFormat::Text,
//         color: Color::Auto,
//         watch_config: false,
//         internal_log_rate_limit: 0,
//         graceful_shutdown_limit_secs: NonZeroU64::new(60).unwrap(),
//
//         no_graceful_shutdown_limit: false,
//         // allocation_tracing: false,
//         // allocation_tracing_reporting_interval_ms: 0,
//         openssl_no_probe: false,
//         allow_empty_config: false,
//     };
//     root.init_global();
//     let app_config = ApplicationConfig::from_opts(&root, &mut signals.handler, Default::default())
//         .await
//         .map_err(|code| AppError::ExitCode { code })?;
//     let application = Application {
//         root_opts: root,
//         config: app_config,
//         signals,
//     };
//     let started_app = application
//         .start(runtime.handle())
//         .map_err(|code| AppError::ExitCode { code })?;
//     let code = started_app.run().await;
//     if code.success() {
//         Ok(())
//     } else {
//         Err(AppError::ExitCode {
//             code: code.code().unwrap(),
//         })
//     }
// }
//
// fn run_app(config: PathBuf) -> anyhow::Result<ExitStatus> {

// }
