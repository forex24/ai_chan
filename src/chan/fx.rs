use pyo3::prelude::*;
use generational_arena::{Arena, Index};
use crate::common::error::{ChanException, ErrCode};
use crate::common::enums::FxType;
use crate::kline::KLineUnit;

/// Represents a fractal (分型) in the Chan system
#[pyclass]
#[derive(Debug)]
pub struct Fx {
    pub fx_type: FxType,                // 分型类型
    pub klc_idx: Index,                 // K线组合的索引
    pub high: f64,                      // 最高价
    pub low: f64,                       // 最低价
    pub pre_fx_idx: Option<Index>,      // 前一个分型的索引
    pub next_fx_idx: Option<Index>,     // 后一个分型的索引
    pub is_real: bool,                  // 是否是真分型
    pub is_strict: bool,                // 是否是严格分型
    pub klu_range: (Index, Index),      // K线范围
}

#[pymethods]
impl Fx {
    /// Create a new Fx instance
    #[new]
    pub fn new(
        fx_type: FxType,
        klc_idx: Index,
        high: f64,
        low: f64,
        klu_range: (Index, Index),
        is_strict: bool
    ) -> Self {
        Self {
            fx_type,
            klc_idx,
            high,
            low,
            pre_fx_idx: None,
            next_fx_idx: None,
            is_real: false,
            is_strict,
            klu_range,
        }
    }

    /// Get the fractal type
    #[getter]
    pub fn get_type(&self) -> FxType {
        self.fx_type
    }

    /// Get the high price
    #[getter]
    pub fn get_high(&self) -> f64 {
        self.high
    }

    /// Get the low price
    #[getter]
    pub fn get_low(&self) -> f64 {
        self.low
    }

    /// Get the K-line combination index
    #[getter]
    pub fn get_klc_idx(&self) -> Index {
        self.klc_idx
    }

    /// Get the previous fractal index
    #[getter]
    pub fn get_pre_fx_idx(&self) -> Option<Index> {
        self.pre_fx_idx
    }

    /// Get the next fractal index
    #[getter]
    pub fn get_next_fx_idx(&self) -> Option<Index> {
        self.next_fx_idx
    }

    /// Check if it's a real fractal
    #[getter]
    pub fn is_real(&self) -> bool {
        self.is_real
    }

    /// Check if it's a strict fractal
    #[getter]
    pub fn is_strict(&self) -> bool {
        self.is_strict
    }

    /// Get the K-line range
    #[getter]
    pub fn get_klu_range(&self) -> (Index, Index) {
        self.klu_range
    }

    /// String representation
    fn __str__(&self) -> String {
        format!("Fx(type={:?}, klc_idx={:?}, high={}, low={}, is_real={}, is_strict={})",
            self.fx_type, self.klc_idx, self.high, self.low, self.is_real, self.is_strict)
    }
}

impl Fx {
    /// Set the previous fractal index
    pub fn set_pre_fx(&mut self, idx: Option<Index>) {
        self.pre_fx_idx = idx;
    }

    /// Set the next fractal index
    pub fn set_next_fx(&mut self, idx: Option<Index>) {
        self.next_fx_idx = idx;
    }

    /// Set the real fractal flag
    pub fn set_real(&mut self, is_real: bool) {
        self.is_real = is_real;
    }

    /// Check if this fractal is valid compared to another one
    pub fn is_valid_with(&self, other: &Fx, arena: &Arena<KLineUnit>) -> PyResult<bool> {
        if self.fx_type == other.fx_type {
            return Ok(false);
        }

        match self.fx_type {
            FxType::Top => {
                if self.high <= other.high {
                    return Ok(false);
                }
            }
            FxType::Bottom => {
                if self.low >= other.low {
                    return Ok(false);
                }
            }
            FxType::Unknown => {
                return Err(ChanException::new(
                    "Unknown fractal type".to_string(),
                    ErrCode::CommonError
                ).into());
            }
        }

        // Check K-line sequence
        let (start1, end1) = self.klu_range;
        let (start2, end2) = other.klu_range;
        
        let start1_klu = arena.get(start1).ok_or_else(|| 
            ChanException::new("Invalid KLineUnit index".to_string(), ErrCode::CommonError))?;
        let end2_klu = arena.get(end2).ok_or_else(|| 
            ChanException::new("Invalid KLineUnit index".to_string(), ErrCode::CommonError))?;

        Ok(start1_klu.get_time() < end2_klu.get_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fx_creation() {
        let fx = Fx::new(
            FxType::Top,
            Index::from_raw_parts(0, 1),
            100.0,
            90.0,
            (Index::from_raw_parts(0, 1), Index::from_raw_parts(1, 1)),
            true
        );

        assert_eq!(fx.get_type(), FxType::Top);
        assert_eq!(fx.get_high(), 100.0);
        assert_eq!(fx.get_low(), 90.0);
        assert!(!fx.is_real());
        assert!(fx.is_strict());
    }

    #[test]
    fn test_fx_setters() {
        let mut fx = Fx::new(
            FxType::Bottom,
            Index::from_raw_parts(0, 1),
            100.0,
            90.0,
            (Index::from_raw_parts(0, 1), Index::from_raw_parts(1, 1)),
            true
        );

        let pre_idx = Index::from_raw_parts(1, 1);
        let next_idx = Index::from_raw_parts(2, 1);

        fx.set_pre_fx(Some(pre_idx));
        fx.set_next_fx(Some(next_idx));
        fx.set_real(true);

        assert_eq!(fx.get_pre_fx_idx(), Some(pre_idx));
        assert_eq!(fx.get_next_fx_idx(), Some(next_idx));
        assert!(fx.is_real());
    }
} 