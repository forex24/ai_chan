use pyo3::prelude::*;
use generational_arena::{Arena, Index};
use crate::common::error::{ChanException, ErrCode};
use crate::bi::Bi;
use crate::kline::KLineUnit;
use crate::seg::Seg;

/// Represents a combined item in the Chan system
#[pyclass]
#[derive(Debug)]
pub struct CombineItem {
    pub time_begin: i64,  // 开始时间
    pub time_end: i64,    // 结束时间
    pub high: f64,        // 最高价
    pub low: f64,         // 最低价
}

#[pymethods]
impl CombineItem {
    /// Create a new CombineItem from various types (Bi, KLineUnit, Seg)
    #[new]
    pub fn new(item: &PyAny, arena: &Arena<PyObject>) -> PyResult<Self> {
        if item.is_instance_of::<Bi>()? {
            let bi: &Bi = item.extract()?;
            Ok(Self::from_bi(bi, arena)?)
        } else if item.is_instance_of::<KLineUnit>()? {
            let klu: &KLineUnit = item.extract()?;
            Ok(Self::from_kline_unit(klu)?)
        } else if item.is_instance_of::<Seg>()? {
            let seg: &Seg = item.extract()?;
            Ok(Self::from_seg(seg, arena)?)
        } else {
            Err(ChanException::new(
                format!("{:?} is unsupported sub class of CombineItem", 
                    item.get_type()),
                ErrCode::CommonError
            ).into())
        }
    }

    /// String representation
    fn __str__(&self) -> String {
        format!("CombineItem(time_begin={}, time_end={}, high={}, low={})",
            self.time_begin, self.time_end, self.high, self.low)
    }

    /// Get the high price
    #[getter]
    fn get_high(&self) -> f64 {
        self.high
    }

    /// Get the low price
    #[getter]
    fn get_low(&self) -> f64 {
        self.low
    }

    /// Get the begin time
    #[getter]
    fn get_time_begin(&self) -> i64 {
        self.time_begin
    }

    /// Get the end time
    #[getter]
    fn get_time_end(&self) -> i64 {
        self.time_end
    }
}

impl CombineItem {
    /// Create a CombineItem from a Bi instance
    fn from_bi(bi: &Bi, arena: &Arena<PyObject>) -> PyResult<Self> {
        Ok(Self {
            time_begin: bi.get_begin_klc_idx(arena)?,
            time_end: bi.get_end_klc_idx(arena)?,
            high: bi.high(),
            low: bi.low(),
        })
    }

    /// Create a CombineItem from a KLineUnit instance
    fn from_kline_unit(klu: &KLineUnit) -> PyResult<Self> {
        Ok(Self {
            time_begin: klu.get_time(),
            time_end: klu.get_time(),
            high: klu.high,
            low: klu.low,
        })
    }

    /// Create a CombineItem from a Seg instance
    fn from_seg(seg: &Seg, arena: &Arena<PyObject>) -> PyResult<Self> {
        Ok(Self {
            time_begin: seg.get_start_bi_begin_time(arena)?,
            time_end: seg.get_end_bi_end_time(arena)?,
            high: seg.high(),
            low: seg.low(),
        })
    }
}

/// Combiner module for managing combined items
#[pyclass]
pub struct Combiner {
    items: Vec<Index>,  // Arena indices for CombineItems
    arena: Arena<CombineItem>,
}

#[pymethods]
impl Combiner {
    /// Create a new Combiner instance
    #[new]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            arena: Arena::new(),
        }
    }

    /// Add a new item to the combiner
    pub fn add_item(&mut self, item: &PyAny, global_arena: &Arena<PyObject>) -> PyResult<()> {
        let combine_item = CombineItem::new(item, global_arena)?;
        let idx = self.arena.insert(combine_item);
        self.items.push(idx);
        Ok(())
    }

    /// Get all items in the combiner
    pub fn get_items(&self) -> Vec<&CombineItem> {
        self.items.iter()
            .filter_map(|&idx| self.arena.get(idx))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kline::KLineUnit;

    #[test]
    fn test_combine_item_from_kline() {
        Python::with_gil(|py| {
            let klu = KLineUnit::new(
                py.eval("{'time': 1234567890, 'high': 100.0, 'low': 90.0}", None, None)
                    .unwrap()
                    .extract()
                    .unwrap(),
                true
            ).unwrap();

            let combine_item = CombineItem::from_kline_unit(&klu).unwrap();
            assert_eq!(combine_item.time_begin, 1234567890);
            assert_eq!(combine_item.time_end, 1234567890);
            assert_eq!(combine_item.high, 100.0);
            assert_eq!(combine_item.low, 90.0);
        });
    }

    #[test]
    fn test_combiner() {
        let mut combiner = Combiner::new();
        assert!(combiner.get_items().is_empty());
    }
} 