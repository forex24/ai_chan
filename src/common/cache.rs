use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use std::any::TypeId;

/// Global cache storage for all instances
static GLOBAL_CACHE: Lazy<RwLock<HashMap<TypeId, HashMap<String, PyObject>>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Cache attribute for method results
#[derive(Debug)]
pub struct CacheAttribute {
    method_name: String,
}

impl CacheAttribute {
    pub fn new(method_name: String) -> Self {
        Self { method_name }
    }

    /// Get cached value for an instance
    pub fn get_cached<T: 'static>(&self, instance: &T) -> Option<PyObject> {
        let cache = GLOBAL_CACHE.read().unwrap();
        let type_cache = cache.get(&TypeId::of::<T>())?;
        type_cache.get(&self.method_name).cloned()
    }

    /// Set cached value for an instance
    pub fn set_cached<T: 'static>(&self, instance: &T, value: PyObject) {
        let mut cache = GLOBAL_CACHE.write().unwrap();
        let type_cache = cache.entry(TypeId::of::<T>()).or_default();
        type_cache.insert(self.method_name.clone(), value);
    }
}

/// Macro to implement caching for methods
#[macro_export]
macro_rules! make_cache {
    ($method:ident) => {
        paste::paste! {
            fn [<cached_ $method>](&self) -> PyResult<PyObject> {
                let cache_attr = CacheAttribute::new(stringify!($method).to_string());
                
                if let Some(cached) = cache_attr.get_cached(self) {
                    return Ok(cached);
                }

                let result = self.$method()?;
                cache_attr.set_cached(self, result.clone());
                Ok(result)
            }
        }
    };
}

/// Python module initialization
#[pymodule]
fn cache(_py: Python, m: &PyModule) -> PyResult<()> {
    Ok(())
}

// Example usage:
/*
#[pymethods]
impl MyClass {
    make_cache!(expensive_calculation);

    fn expensive_calculation(&self) -> PyResult<PyObject> {
        // Actual calculation here
        Python::with_gil(|py| {
            Ok(42.into_py(py))
        })
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::Python;

    #[test]
    fn test_cache_attribute() {
        Python::with_gil(|py| {
            struct TestStruct;
            
            let cache_attr = CacheAttribute::new("test_method".to_string());
            let instance = TestStruct;
            let value = 42.into_py(py);
            
            // Initially no cached value
            assert!(cache_attr.get_cached(&instance).is_none());
            
            // Set and get cached value
            cache_attr.set_cached(&instance, value.clone());
            let cached = cache_attr.get_cached(&instance).unwrap();
            assert_eq!(cached.extract::<i32>(py).unwrap(), 42);
        });
    }
} 