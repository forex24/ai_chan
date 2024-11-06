use pyo3::prelude::*;
use crate::common::enums::{KLineType, BiDir};

/// Check if the given K-line type is less than day level
#[pyfunction]
pub fn kltype_lt_day(ktype: KLineType) -> bool {
    (ktype as i32) < (KLineType::KDay as i32)
}

/// Check if the given K-line type is less than or equal to day level
#[pyfunction]
pub fn kltype_lte_day(ktype: KLineType) -> bool {
    (ktype as i32) <= (KLineType::KDay as i32)
}

/// Check if the K-line type list is in descending order (from larger to smaller)
#[pyfunction]
pub fn check_kltype_order(type_list: Vec<KLineType>) -> PyResult<()> {
    if type_list.is_empty() {
        return Ok(());
    }

    let mut last_lv = type_list[0] as i32;
    for &kl_type in &type_list[1..] {
        if (kl_type as i32) >= last_lv {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "K-line type list must be ordered from larger to smaller level"
            ));
        }
        last_lv = kl_type as i32;
    }
    Ok(())
}

/// Revert the BI direction (UP -> DOWN, DOWN -> UP)
#[pyfunction]
pub fn revert_bi_dir(dir: BiDir) -> BiDir {
    match dir {
        BiDir::Up => BiDir::Down,
        BiDir::Down => BiDir::Up,
    }
}

/// Check if two ranges have overlap
#[pyfunction]
pub fn has_overlap(l1: f64, h1: f64, l2: f64, h2: f64, equal: bool) -> bool {
    if equal {
        h2 >= l1 && h1 >= l2
    } else {
        h2 > l1 && h1 > l2
    }
}

/// Convert string to float, return 0.0 if conversion fails
#[pyfunction]
pub fn str2float(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0)
}

/// Parse infinity values for string representation
#[pyfunction]
pub fn parse_inf(v: PyObject, py: Python) -> PyResult<PyObject> {
    if let Ok(f) = v.extract::<f64>(py) {
        if f == f64::INFINITY {
            return Ok(py.eval("float('inf')", None, None)?.into());
        }
        if f == f64::NEG_INFINITY {
            return Ok(py.eval("float('-inf')", None, None)?.into());
        }
    }
    Ok(v)
}

/// Module initialization
#[pymodule]
fn func_util(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(kltype_lt_day, m)?)?;
    m.add_function(wrap_pyfunction!(kltype_lte_day, m)?)?;
    m.add_function(wrap_pyfunction!(check_kltype_order, m)?)?;
    m.add_function(wrap_pyfunction!(revert_bi_dir, m)?)?;
    m.add_function(wrap_pyfunction!(has_overlap, m)?)?;
    m.add_function(wrap_pyfunction!(str2float, m)?)?;
    m.add_function(wrap_pyfunction!(parse_inf, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kltype_lt_day() {
        assert!(kltype_lt_day(KLineType::K1M));
        assert!(!kltype_lt_day(KLineType::KDay));
        assert!(!kltype_lt_day(KLineType::KWeek));
    }

    #[test]
    fn test_kltype_lte_day() {
        assert!(kltype_lte_day(KLineType::K1M));
        assert!(kltype_lte_day(KLineType::KDay));
        assert!(!kltype_lte_day(KLineType::KWeek));
    }

    #[test]
    fn test_check_kltype_order() {
        // Valid order
        assert!(check_kltype_order(vec![
            KLineType::KWeek,
            KLineType::KDay,
            KLineType::K1M
        ]).is_ok());

        // Invalid order
        assert!(check_kltype_order(vec![
            KLineType::K1M,
            KLineType::KDay,
            KLineType::KWeek
        ]).is_err());
    }

    #[test]
    fn test_revert_bi_dir() {
        assert_eq!(revert_bi_dir(BiDir::Up), BiDir::Down);
        assert_eq!(revert_bi_dir(BiDir::Down), BiDir::Up);
    }

    #[test]
    fn test_has_overlap() {
        assert!(has_overlap(1.0, 3.0, 2.0, 4.0, true));
        assert!(has_overlap(1.0, 3.0, 2.0, 4.0, false));
        assert!(has_overlap(1.0, 3.0, 3.0, 4.0, true));
        assert!(!has_overlap(1.0, 3.0, 3.0, 4.0, false));
        assert!(!has_overlap(1.0, 2.0, 3.0, 4.0, true));
    }

    #[test]
    fn test_str2float() {
        assert_eq!(str2float("123.45"), 123.45);
        assert_eq!(str2float("invalid"), 0.0);
    }
} 