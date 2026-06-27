use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use syon_parser::Value;

fn value_to_py(py: Python<'_>, val: &Value) -> PyResult<PyObject> {
    match val {
        Value::Scalar(s) => Ok(s.into_py(py)),
        Value::LiteralBlock(s) => Ok(s.into_py(py)),
        Value::Mapping(entries) => {
            let dict = PyDict::new_bound(py);
            for entry in entries {
                let v = value_to_py(py, &entry.value)?;
                dict.set_item(&entry.key, v)?;
            }
            Ok(dict.into())
        }
        Value::Sequence(items) => {
            let list = PyList::empty_bound(py);
            for item in items {
                list.append(value_to_py(py, &item.value)?)?;
            }
            Ok(list.into())
        }
    }
}

#[pyfunction]
fn parse(py: Python<'_>, input: &str) -> PyResult<PyObject> {
    let file = syon_parser::parse(input).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let first = file.documents.into_iter().next()
        .ok_or_else(|| PyValueError::new_err("no documents"))?;
    value_to_py(py, &first.body)
}

#[pymodule]
fn syon(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}
