use cpython::*;
use std::cell::Cell;
use std::sync::Arc;
use tokio::runtime;
use wickdl::{ServiceState, EncryptedPak as NativeEncPak, PakService as NativePak};

py_module_initializer!(pywickdl, |py, m| {
    m.add(py, "__doc__", "An asynchronous partial EGS downloader")?;
    m.add_class::<Runtime>(py)?;
    m.add_class::<WickService>(py)?;
    m.add_class::<EncryptedPak>(py)?;
    m.add_class::<PakService>(py)?;
    Ok(())
});

py_exception!(pywickdl, WickError);

fn run_callback<T: ToPyObject>(py: Python, eloop: PyObject, cb: PyObject, args: T) {
    eloop.call_method(py, "call_soon_threadsafe", (cb, args), None).unwrap();
}

fn custom_res<T>(py: Python, s: String) -> PyResult<T> {
    Err(PyErr::new::<WickError, _>(py, s))
}

fn custom_err(py: Python, s: String) -> PyErr {
    PyErr::new::<WickError, _>(py, s)
}

py_class!(class Runtime |py| {
    data runtime: runtime::Runtime;

    def __new__(_cls) -> PyResult<Runtime> {
        let rt = runtime::Builder::new()
            .enable_all()
            .threaded_scheduler()
            .build().unwrap();
        
            Runtime::create_instance(py, rt)
    }
    
    def create_service(&self, eloop: PyObject, cb: PyObject) -> PyResult<PyObject> {
        let rt = self.runtime(py);
        rt.spawn(async move {
            let res = ServiceState::new().await;
            let guard = Python::acquire_gil();
            let py = guard.python();
            match res {
                Ok(service) => {
                    let inst = WickService::create_instance(py, Arc::new(service)).unwrap();
                    run_callback(py, eloop, cb, inst);
                },
                Err(err) => {
                    let err = custom_err(py, format!("Error: {}", err)).instance(py);
                    run_callback(py, eloop, cb, err);
                },
            }
        });
        Ok(py.None())
    }
});

py_class!(class EncryptedPak |py| {
    data pak: Cell<Option<NativeEncPak>>;

    def decrypt(&self, rt: Runtime, service: WickService, key: PyString, eloop: PyObject, cb: PyObject) -> PyResult<PyObject> {
        let rt = rt.runtime(py);
        let pak = self.pak(py).replace(None);
        let pak = match pak {
            Some(p) => p,
            None => return custom_res(py, format!("Cannot decrypt consumed pak")),
        };
        let key = key.to_string(py)?.to_string();
        let service = Arc::clone(service.service(py));

        rt.spawn(async move {
            let res = service.decrypt_pak(pak, key).await;
            let guard = Python::acquire_gil();
            let py = guard.python();
            match res {
                Ok(pak) => {
                    let inst = PakService::create_instance(py, Arc::new(pak)).unwrap();
                    run_callback(py, eloop, cb, inst);
                },
                Err(err) => {
                    let err = custom_err(py, format!("Error: {}", err)).instance(py);
                    run_callback(py, eloop, cb, err);
                },
            }
        });
        Ok(py.None())
    }
});

py_class!(class PakService |py| {
    data pak: Arc<NativePak>;

    def get_file_data(&self, rt: Runtime, file: PyString, eloop: PyObject, cb: PyObject) -> PyResult<PyObject> {
        let rt = rt.runtime(py);
        let file = file.to_string(py)?.to_string();
        let pak = Arc::clone(self.pak(py));
        rt.spawn(async move {
            let res = pak.get_data(&file).await;
            let guard = Python::acquire_gil();
            let py = guard.python();
            match res {
                Ok(data) => {
                    let bytes = PyBytes::new(py, &data);
                    run_callback(py, eloop, cb, bytes);
                },
                Err(err) => {
                    let err = custom_err(py, format!("Error: {}", err)).instance(py);
                    run_callback(py, eloop, cb, err);
                },
            }
        });
        Ok(py.None())
    }

    def get_pak_mount(&self) -> PyResult<String> {
        let pak = self.pak(py);
        Ok(pak.get_mount_point().to_owned())
    }

    def get_file_names(&self) -> PyResult<PyList> {
        let filenames: Vec<PyObject> = self.pak(py).get_files().into_iter().map(|v| v.to_py_object(py).into_object()).collect();
        Ok(PyList::new(py, filenames.as_slice()))
    }
});

py_class!(class WickService |py| {
    data service: Arc<ServiceState>;

    def __new__(_cls, app_manifest: PyString, chunk_manifest: PyString) -> PyResult<WickService> {
        let app_manifest = app_manifest.to_string(py)?;
        let chunk_manifest = chunk_manifest.to_string(py)?;
        let service = match ServiceState::from_manifests(&app_manifest, &chunk_manifest) {
            Ok(s) => s,
            Err(err) => return custom_res(py, format!("Error: {}", err))
        };

        WickService::create_instance(py, Arc::new(service))
    }

    def get_paks(&self) -> PyResult<PyList> {
        let paknames: Vec<PyObject> = self.service(py).get_paks().into_iter().map(|v| v.to_py_object(py).into_object()).collect();
        Ok(PyList::new(py, paknames.as_slice()))
    }

    def download_pak(&self, rt: Runtime, pak: PyString, target: PyString, eloop: PyObject, cb: PyObject) -> PyResult<PyObject> {
        let rt = rt.runtime(py);
        let pakname = pak.to_string(py)?.to_string();
        let target = target.to_string(py)?.to_string();
        let service = Arc::clone(self.service(py));

        rt.spawn(async move {
            let res = service.download_pak(pakname, target).await;
            let guard = Python::acquire_gil();
            let py = guard.python();
            match res {
                Ok(_) => {
                    run_callback(py, eloop, cb, py.None());
                },
                Err(err) => {
                    let err = custom_err(py, format!("Error: {}", err)).instance(py);
                    run_callback(py, eloop, cb, err);
                },
            }
        });
        Ok(py.None())
    }

    def get_pak(&self, rt: Runtime, pak: PyString, eloop: PyObject, cb: PyObject) -> PyResult<PyObject> {
        let rt = rt.runtime(py);
        let pakname = pak.to_string(py)?.to_string();
        let service = Arc::clone(self.service(py));

        rt.spawn(async move {
            let res = service.get_pak(pakname).await;
            let guard = Python::acquire_gil();
            let py = guard.python();
            match res {
                Ok(pak) => {
                    let pak = EncryptedPak::create_instance(py, Cell::new(Some(pak))).unwrap();
                    run_callback(py, eloop, cb, pak);
                },
                Err(err) => {
                    let err = custom_err(py, format!("Error: {}", err)).instance(py);
                    run_callback(py, eloop, cb, err);
                },
            }
        });

        Ok(py.None())
    }
});