use pyo3::prelude::*;
use generational_arena::{Arena, Index};
use crate::common::enums::{FxType, KLineDir, FxCheckMethod, KLineType};
use crate::common::error::{ChanException, ErrCode};
use crate::kline::kline_unit::KLineUnit;
use crate::common::func_util::has_overlap;
use std::cmp::{max, min};

/// Represents a combined K-line in the Chan system
#[pyclass]
#[derive(Debug)]
pub struct KLine {
    #[pyo3(get)]
    pub idx: usize,                    // K线索引，使用usize更符合Rust的索引类型
    pub kl_type: KLineType,          // K线类型
    pub dir: KLineDir,               // K线方向
    pub fx: FxType,                  // 分型类型
    pub high: f64,                   // 最高价
    pub low: f64,                    // 最低价
    pub time_begin: i64,             // 开始时间
    pub time_end: i64,               // 结束时间
    pub units: Vec<Index>,           // K线单元索引列表
    pub pre_kl: Option<Index>,       // 前一个K线索引
    pub next_kl: Option<Index>,      // 后一个K线索引
}

#[pymethods]
impl KLine {
    /// Create a new KLine instance
    #[new]
    pub fn new(kl_unit: &KLineUnit, idx: usize, dir: KLineDir) -> PyResult<Self> {
        Ok(Self {
            idx,
            kl_type: kl_unit.kl_type,
            dir,
            fx: FxType::Unknown,
            high: kl_unit.high,
            low: kl_unit.low,
            time_begin: kl_unit.time,
            time_end: kl_unit.time,
            units: vec![],
            pre_kl: None,
            next_kl: None,
        })
    }

    /// String representation
    fn __str__(&self) -> String {
        let fx_token = match self.fx {
            FxType::Top => "^",
            FxType::Bottom => "_",
            FxType::Unknown => "",
        };
        format!("{}th{}:{}~{}({:?}|{}) low={} high={}",
            self.idx, fx_token, self.time_begin, self.time_end, 
            self.kl_type, self.units.len(), self.low, self.high)
    }

    /// Get all getters
    #[getter]
    fn get_kl_type(&self) -> KLineType {
        self.kl_type
    }

    #[getter]
    fn get_dir(&self) -> KLineDir {
        self.dir
    }

    #[getter]
    fn get_fx(&self) -> FxType {
        self.fx
    }

    #[getter]
    fn get_high(&self) -> f64 {
        self.high
    }

    #[getter]
    fn get_low(&self) -> f64 {
        self.low
    }

    #[getter]
    fn get_time_begin(&self) -> i64 {
        self.time_begin
    }

    #[getter]
    fn get_time_end(&self) -> i64 {
        self.time_end
    }

    #[getter]
    fn get_units(&self) -> Vec<Index> {
        self.units.clone()
    }

    #[getter]
    fn get_pre_kl(&self) -> Option<Index> {
        self.pre_kl
    }

    #[getter]
    fn get_next_kl(&self) -> Option<Index> {
        self.next_kl
    }

    /// Get sub KLine combinations
    pub fn get_sub_klc(&self, arena: &Arena<KLineUnit>) -> Vec<Index> {
        let mut result = Vec::new();
        let mut last_klc = None;

        for &unit_idx in &self.units {
            if let Some(unit) = arena.get(unit_idx) {
                for child_idx in unit.get_children() {
                    if let Some(child) = arena.get(child_idx) {
                        if let Some(klc_idx) = child.get_klc_idx() {
                            if last_klc != Some(klc_idx) {
                                last_klc = Some(klc_idx);
                                result.push(klc_idx);
                            }
                        }
                    }
                }
            }
        }
        result
    }

    /// Get maximum high price from K-line units
    pub fn get_klu_max_high(&self, arena: &Arena<KLineUnit>) -> f64 {
        self.units.iter()
            .filter_map(|&idx| arena.get(idx))
            .map(|unit| unit.high)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Get minimum low price from K-line units
    pub fn get_klu_min_low(&self, arena: &Arena<KLineUnit>) -> f64 {
        self.units.iter()
            .filter_map(|&idx| arena.get(idx))
            .map(|unit| unit.low)
            .fold(f64::INFINITY, f64::min)
    }

    /// Check if there's a gap with the next K-line
    pub fn has_gap_with_next(&self, arena: &Arena<KLineUnit>) -> PyResult<bool> {
        if let Some(next_idx) = self.next_kl {
            if let Some(next) = arena.get(next_idx) {
                return Ok(!has_overlap(
                    self.get_klu_min_low(arena),
                    self.get_klu_max_high(arena),
                    next.get_klu_min_low(arena),
                    next.get_klu_max_high(arena),
                    true
                ));
            }
        }
        Err(ChanException::new("Next K-line not found".to_string(), ErrCode::CommonError).into())
    }

    /// Check if the fractal is valid
    pub fn check_fx_valid(&self, item2: &KLine, method: FxCheckMethod, for_virtual: bool, 
        arena: &Arena<KLine>) -> PyResult<bool> {
        
        // 基本检查
        if self.next_kl.is_none() || item2.pre_kl.is_none() || self.pre_kl.is_none() {
            return Err(ChanException::new(
                "Invalid K-line sequence".to_string(),
                ErrCode::CommonError
            ).into());
        }

        if item2.idx <= self.idx {
            return Err(ChanException::new(
                "Invalid K-line order".to_string(),
                ErrCode::CommonError
            ).into());
        }

        match self.fx {
            FxType::Top => {
                if !for_virtual && item2.fx != FxType::Bottom {
                    return Ok(false);
                }
                if for_virtual && item2.dir != KLineDir::Down {
                    return Ok(false);
                }

                let (item2_high, self_low) = match method {
                    FxCheckMethod::Half => {
                        let item2_high = max(
                            item2.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.high).unwrap_or(f64::NEG_INFINITY),
                            item2.high
                        );
                        let self_low = min(
                            self.low,
                            self.next_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.low).unwrap_or(f64::INFINITY)
                        );
                        (item2_high, self_low)
                    },
                    FxCheckMethod::Loss => {
                        (item2.high, self.low)
                    },
                    FxCheckMethod::Strict | FxCheckMethod::Totally => {
                        let item2_high = if for_virtual {
                            max(
                                item2.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.high).unwrap_or(f64::NEG_INFINITY),
                                item2.high
                            )
                        } else {
                            let next_high = item2.next_kl.and_then(|idx| arena.get(idx))
                                .map(|kl| kl.high)
                                .unwrap_or(f64::NEG_INFINITY);
                            max(max(
                                item2.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.high).unwrap_or(f64::NEG_INFINITY),
                                item2.high
                            ), next_high)
                        };
                        let self_low = min(min(
                            self.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.low).unwrap_or(f64::INFINITY),
                            self.low
                        ), self.next_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.low).unwrap_or(f64::INFINITY));
                        (item2_high, self_low)
                    },
                };

                if method == FxCheckMethod::Totally {
                    Ok(self.low > item2_high)
                } else {
                    Ok(self.high > item2_high && item2.low < self_low)
                }
            },
            FxType::Bottom => {
                if !for_virtual && item2.fx != FxType::Top {
                    return Ok(false);
                }
                if for_virtual && item2.dir != KLineDir::Up {
                    return Ok(false);
                }

                let (item2_low, self_high) = match method {
                    FxCheckMethod::Half => {
                        let item2_low = min(
                            item2.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.low).unwrap_or(f64::INFINITY),
                            item2.low
                        );
                        let self_high = max(
                            self.high,
                            self.next_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.high).unwrap_or(f64::NEG_INFINITY)
                        );
                        (item2_low, self_high)
                    },
                    FxCheckMethod::Loss => {
                        (item2.low, self.high)
                    },
                    FxCheckMethod::Strict | FxCheckMethod::Totally => {
                        let item2_low = if for_virtual {
                            min(
                                item2.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.low).unwrap_or(f64::INFINITY),
                                item2.low
                            )
                        } else {
                            let next_low = item2.next_kl.and_then(|idx| arena.get(idx))
                                .map(|kl| kl.low)
                                .unwrap_or(f64::INFINITY);
                            min(min(
                                item2.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.low).unwrap_or(f64::INFINITY),
                                item2.low
                            ), next_low)
                        };
                        let self_high = max(max(
                            self.pre_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.high).unwrap_or(f64::NEG_INFINITY),
                            self.high
                        ), self.next_kl.and_then(|idx| arena.get(idx)).map(|kl| kl.high).unwrap_or(f64::NEG_INFINITY));
                        (item2_low, self_high)
                    },
                };

                if method == FxCheckMethod::Totally {
                    Ok(self.high < item2_low)
                } else {
                    Ok(self.low < item2_low && item2.high > self_high)
                }
            },
            FxType::Unknown => {
                Err(ChanException::new(
                    "Only top/bottom fx can check_valid_top_button".to_string(),
                    ErrCode::BiErr
                ).into())
            }
        }
    }
}

impl KLine {
    /// Add a K-line unit to this K-line
    pub fn add_unit(&mut self, unit_idx: Index, arena: &Arena<KLineUnit>) -> PyResult<()> {
        if let Some(unit) = arena.get(unit_idx) {
            self.units.push(unit_idx);
            self.high = f64::max(self.high, unit.high);
            self.low = f64::min(self.low, unit.low);
            self.time_end = unit.time;
            Ok(())
        } else {
            Err(ChanException::new(
                "Invalid KLineUnit index".to_string(),
                ErrCode::CommonError
            ).into())
        }
    }

    /// Set the fractal type
    pub fn set_fx(&mut self, fx_type: FxType) {
        self.fx = fx_type;
    }

    /// Link to the next K-line
    pub fn set_next(&mut self, next_idx: Option<Index>) {
        self.next_kl = next_idx;
    }

    /// Link to the previous K-line
    pub fn set_pre(&mut self, pre_idx: Option<Index>) {
        self.pre_kl = pre_idx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kline_creation() {
        let unit = KLineUnit::new_test(1234567890, 100.0, 105.0, 110.0, 95.0);
        let kline = KLine::new(&unit, 1, KLineDir::Up).unwrap();
        
        assert_eq!(kline.idx, 1);
        assert_eq!(kline.high, 110.0);
        assert_eq!(kline.low, 95.0);
        assert_eq!(kline.time_begin, 1234567890);
        assert_eq!(kline.time_end, 1234567890);
    }

    #[test]
    fn test_kline_fx_validation() {
        let mut arena = Arena::new();
        
        // Create test K-lines with usize indices
        let mut kl1 = KLine::new_test(0, 100.0, 90.0);  // 使用0作为第一个索引
        let mut kl2 = KLine::new_test(1, 95.0, 85.0);   // 使用1作为第二个索引
        let mut kl3 = KLine::new_test(2, 110.0, 100.0); // 使用2作为第三个索引
        
        // Set up relationships
        kl1.set_fx(FxType::Top);
        kl3.set_fx(FxType::Bottom);
        
        let idx1 = arena.insert(kl1);
        let idx2 = arena.insert(kl2);
        let idx3 = arena.insert(kl3);
        
        if let Some(kl1) = arena.get_mut(idx1) {
            kl1.set_next(Some(idx2));
        }
        if let Some(kl2) = arena.get_mut(idx2) {
            kl2.set_pre(Some(idx1));
            kl2.set_next(Some(idx3));
        }
        if let Some(kl3) = arena.get_mut(idx3) {
            kl3.set_pre(Some(idx2));
        }
        
        // Test validation
        let kl1 = arena.get(idx1).unwrap();
        let kl3 = arena.get(idx3).unwrap();
        assert!(kl1.check_fx_valid(kl3, FxCheckMethod::Strict, false, &arena).unwrap());
    }

    #[test]
    fn test_gap_detection() {
        let mut arena = Arena::new();
        
        let mut kl1 = KLine::new_test(1, 100.0, 90.0);
        let mut kl2 = KLine::new_test(2, 120.0, 110.0);
        
        let idx1 = arena.insert(kl1);
        let idx2 = arena.insert(kl2);
        
        if let Some(kl1) = arena.get_mut(idx1) {
            kl1.set_next(Some(idx2));
        }
        if let Some(kl2) = arena.get_mut(idx2) {
            kl2.set_pre(Some(idx1));
        }
        
        let kl1 = arena.get(idx1).unwrap();
        assert!(kl1.has_gap_with_next(&arena).unwrap());
    }
} 