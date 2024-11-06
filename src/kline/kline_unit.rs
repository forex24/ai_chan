use pyo3::prelude::*;
use generational_arena::{Arena, Index};
use std::collections::HashMap;
use crate::common::enums::{KLineType, KLineDir};
use crate::common::error::{ChanException, ErrCode};
use crate::common::trade_info::TradeInfo;

/// Represents a basic K-line unit in the Chan system
#[pyclass]
#[derive(Debug)]
pub struct KLineUnit {
    pub time: i64,           // K线时间
    pub open: f64,           // 开盘价
    pub close: f64,         // 收盘价
    pub high: f64,          // 最高价
    pub low: f64,           // 最低价
    pub kl_type: KLineType, // K线类型
    pub dir: KLineDir,      // K线方向
    pub trade_info: TradeInfo, // 交易信息
    pub parent_idx: Option<Index>, // 父K线索引
    pub children: Vec<Index>,     // 子K线索引列表
    pub klc_idx: Option<Index>,   // 所属K线组合索引
}

#[pymethods]
impl KLineUnit {
    /// Create a new KLineUnit instance
    #[new]
    pub fn new(data: &PyDict, auto_trend: bool) -> PyResult<Self> {
        let time = data.get_item("time")?.extract::<i64>()?;
        let open = data.get_item("open")?.extract::<f64>()?;
        let close = data.get_item("close")?.extract::<f64>()?;
        let high = data.get_item("high")?.extract::<f64>()?;
        let low = data.get_item("low")?.extract::<f64>()?;
        let kl_type = data.get_item("kl_type")
            .map(|v| v.extract::<KLineType>())
            .transpose()?
            .unwrap_or(KLineType::KDay);

        let dir = if auto_trend {
            if close >= open {
                KLineDir::Up
            } else {
                KLineDir::Down
            }
        } else {
            KLineDir::Up
        };

        let mut trade_dict = HashMap::new();
        if let Some(volume) = data.get_item("volume") {
            trade_dict.insert("volume".to_string(), volume.to_object(data.py()));
        }
        if let Some(turnover) = data.get_item("turnover") {
            trade_dict.insert("turnover".to_string(), turnover.to_object(data.py()));
        }
        if let Some(turnrate) = data.get_item("turnrate") {
            trade_dict.insert("turnrate".to_string(), turnrate.to_object(data.py()));
        }

        Ok(Self {
            time,
            open,
            close,
            high,
            low,
            kl_type,
            dir,
            trade_info: TradeInfo::new(trade_dict)?,
            parent_idx: None,
            children: Vec::new(),
            klc_idx: None,
        })
    }

    /// Get the K-line time
    #[getter]
    pub fn get_time(&self) -> i64 {
        self.time
    }

    /// Get the K-line type
    #[getter]
    pub fn get_kl_type(&self) -> KLineType {
        self.kl_type
    }

    /// Get the K-line direction
    #[getter]
    pub fn get_dir(&self) -> KLineDir {
        self.dir
    }

    /// Get the trade information
    #[getter]
    pub fn get_trade_info(&self) -> &TradeInfo {
        &self.trade_info
    }

    /// Get the parent K-line index
    #[getter]
    pub fn get_parent_idx(&self) -> Option<Index> {
        self.parent_idx
    }

    /// Get the children K-line indices
    #[getter]
    pub fn get_children(&self) -> Vec<Index> {
        self.children.clone()
    }

    /// Get the K-line combination index
    #[getter]
    pub fn get_klc_idx(&self) -> Option<Index> {
        self.klc_idx
    }

    /// Set the K-line combination index
    pub fn set_klc(&mut self, idx: Index) {
        self.klc_idx = Some(idx);
    }

    /// Add a child K-line index
    pub fn add_child(&mut self, child_idx: Index) {
        self.children.push(child_idx);
    }

    /// Set the parent K-line index
    pub fn set_parent(&mut self, parent_idx: Index) {
        self.parent_idx = Some(parent_idx);
    }

    /// String representation
    fn __str__(&self) -> String {
        format!("KLineUnit(time={}, open={}, close={}, high={}, low={}, type={:?}, dir={:?})",
            self.time, self.open, self.close, self.high, self.low, self.kl_type, self.dir)
    }
}

impl KLineUnit {
    /// Check if this K-line unit contains another one
    pub fn contains(&self, other: &KLineUnit) -> bool {
        self.high >= other.high && self.low <= other.low
    }

    /// Check if this K-line unit has overlap with another one
    pub fn has_overlap(&self, other: &KLineUnit) -> bool {
        !(self.high < other.low || self.low > other.high)
    }

    /// Get all descendant K-line units
    pub fn get_all_descendants(&self, arena: &Arena<KLineUnit>) -> Vec<Index> {
        let mut result = Vec::new();
        let mut stack = self.children.clone();

        while let Some(idx) = stack.pop() {
            if let Some(child) = arena.get(idx) {
                result.push(idx);
                stack.extend(child.children.iter());
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::Python;

    #[test]
    fn test_kline_unit_creation() {
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("time", 1234567890).unwrap();
            dict.set_item("open", 100.0).unwrap();
            dict.set_item("close", 105.0).unwrap();
            dict.set_item("high", 110.0).unwrap();
            dict.set_item("low", 95.0).unwrap();

            let klu = KLineUnit::new(dict, true).unwrap();
            assert_eq!(klu.time, 1234567890);
            assert_eq!(klu.open, 100.0);
            assert_eq!(klu.close, 105.0);
            assert_eq!(klu.high, 110.0);
            assert_eq!(klu.low, 95.0);
            assert_eq!(klu.dir, KLineDir::Up);
        });
    }

    #[test]
    fn test_kline_unit_relationships() {
        let mut arena = Arena::new();
        let parent_idx = arena.insert(KLineUnit::new_empty());
        let child_idx = arena.insert(KLineUnit::new_empty());

        if let Some(parent) = arena.get_mut(parent_idx) {
            parent.add_child(child_idx);
        }
        if let Some(child) = arena.get_mut(child_idx) {
            child.set_parent(parent_idx);
        }

        let parent = arena.get(parent_idx).unwrap();
        assert_eq!(parent.children[0], child_idx);
        let child = arena.get(child_idx).unwrap();
        assert_eq!(child.parent_idx.unwrap(), parent_idx);
    }
} 