use pyo3::prelude::*;
use generational_arena::{Arena, Index};
use std::collections::HashMap;
use crate::common::enums::{FxType, KLineDir, BiDir, MacdAlgo};
use crate::common::error::{ChanException, ErrCode};
use crate::kline::KLine;
use crate::bi::{Bi, BiConfig};
use crate::common::func_util::{has_overlap, get_macd_metrics};

/// Manages a list of Bi (笔) in the Chan system
#[pyclass]
#[derive(Debug)]
pub struct BiList {
    pub bi_list: Vec<Index>,         // 笔索引列表
    pub last_end: Option<Index>,     // 最后一笔的尾部K线索引
    pub config: BiConfig,            // 笔的配置
    pub free_klc_lst: Vec<Index>,    // 第一笔未画出来之前的缓存K线索引列表
    arena: Arena<Bi>,                // 笔对象管理
    kline_arena: Arena<KLine>,       // K线对象管理
}

#[pymethods]
impl BiList {
    /// Create a new BiList instance
    #[new]
    pub fn new(bi_conf: BiConfig) -> Self {
        Self {
            bi_list: Vec::new(),
            last_end: None,
            config: bi_conf,
            free_klc_lst: Vec::new(),
            arena: Arena::new(),
            kline_arena: Arena::new(),
        }
    }

    /// String representation
    fn __str__(&self) -> String {
        self.bi_list.iter()
            .filter_map(|&idx| self.arena.get(idx))
            .map(|bi| bi.__str__(&self.kline_arena).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get length of bi list
    fn __len__(&self) -> usize {
        self.bi_list.len()
    }

    /// Iterator implementation
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<&PyAny> {
        let iter = slf.bi_list.iter()
            .filter_map(|&idx| slf.arena.get(idx))
            .map(|bi| bi.to_object(slf.py()));
        Ok(iter.into_ref(slf.py()))
    }

    /// Get bi by index
    fn __getitem__(&self, index: PyObject, py: Python) -> PyResult<PyObject> {
        if let Ok(idx) = index.extract::<usize>(py) {
            if idx >= self.bi_list.len() {
                return Err(PyIndexError::new_err("Index out of range"));
            }
            if let Some(bi) = self.arena.get(self.bi_list[idx]) {
                return Ok(bi.to_object(py));
            }
        } else if let Ok(slice) = index.extract::<PySlice>(py) {
            let indices = slice.indices(self.bi_list.len() as i64)?;
            let result: Vec<_> = (indices.start..indices.stop)
                .step_by(indices.step as usize)
                .filter_map(|i| self.arena.get(self.bi_list[i as usize]))
                .map(|bi| bi.to_object(py))
                .collect();
            return Ok(result.to_object(py));
        }
        Err(PyTypeError::new_err("Invalid index type"))
    }

    /// Try to create first bi
    pub fn try_create_first_bi(&mut self, klc: &KLine) -> PyResult<bool> {
        for &exist_free_klc_idx in &self.free_klc_lst {
            if let Some(exist_free_klc) = self.kline_arena.get(exist_free_klc_idx) {
                if exist_free_klc.fx == klc.fx {
                    continue;
                }
                if self.can_make_bi(klc, exist_free_klc)? {
                    self.add_new_bi(exist_free_klc_idx, klc.idx)?;
                    self.last_end = Some(klc.idx);
                    return Ok(true);
                }
            }
        }
        self.free_klc_lst.push(klc.idx);
        self.last_end = Some(klc.idx);
        Ok(false)
    }

    /// Update bi
    pub fn update_bi(&mut self, klc: &KLine, last_klc: &KLine, cal_virtual: bool) -> PyResult<bool> {
        let flag1 = self.update_bi_sure(klc)?;
        if cal_virtual {
            let flag2 = self.try_add_virtual_bi(last_klc)?;
            Ok(flag1 || flag2)
        } else {
            Ok(flag1)
        }
    }

    /// Check if can update peak
    pub fn can_update_peak(&self, klc: &KLine) -> PyResult<bool> {
        if self.config.bi_allow_sub_peak || self.bi_list.len() < 2 {
            return Ok(false);
        }

        let last_bi = self.arena.get(self.bi_list[self.bi_list.len() - 1])
            .ok_or_else(|| ChanException::new(
                "Invalid bi index".to_string(),
                ErrCode::CommonError
            ))?;

        let last_begin_klc = self.kline_arena.get(last_bi.begin_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid kline index".to_string(),
                ErrCode::CommonError
            ))?;

        if last_bi.dir == BiDir::Down && klc.high < last_begin_klc.high {
            return Ok(false);
        }
        if last_bi.dir == BiDir::Up && klc.low > last_begin_klc.low {
            return Ok(false);
        }

        if !end_is_peak(last_begin_klc, klc, &self.kline_arena)? {
            return Ok(false);
        }

        let second_last_bi = self.arena.get(self.bi_list[self.bi_list.len() - 2])
            .ok_or_else(|| ChanException::new(
                "Invalid bi index".to_string(),
                ErrCode::CommonError
            ))?;

        let second_last_begin_klc = self.kline_arena.get(second_last_bi.begin_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid kline index".to_string(),
                ErrCode::CommonError
            ))?;

        if last_bi.dir == BiDir::Down && last_bi.get_end_val()? < second_last_begin_klc.low {
            return Ok(false);
        }
        if last_bi.dir == BiDir::Up && last_bi.get_end_val()? > second_last_begin_klc.high {
            return Ok(false);
        }

        Ok(true)
    }

    /// Add a new bi
    pub fn add_new_bi(&mut self, begin_klc_idx: Index, end_klc_idx: Index) -> PyResult<()> {
        let begin_klc = self.kline_arena.get(begin_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid begin kline index".to_string(),
                ErrCode::CommonError
            ))?;
        let end_klc = self.kline_arena.get(end_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid end kline index".to_string(),
                ErrCode::CommonError
            ))?;

        let bi = Bi::new(begin_klc, end_klc, self.bi_list.len(), true, &self.kline_arena)?;
        let bi_idx = self.arena.insert(bi);

        if let Some(last_bi_idx) = self.bi_list.last() {
            if let Some(last_bi) = self.arena.get_mut(*last_bi_idx) {
                last_bi.next = Some(bi_idx);
            }
            if let Some(new_bi) = self.arena.get_mut(bi_idx) {
                new_bi.pre = Some(*last_bi_idx);
            }
        }

        self.bi_list.push(bi_idx);
        Ok(())
    }

    /// Try to add virtual bi
    pub fn try_add_virtual_bi(&mut self, last_klc: &KLine) -> PyResult<bool> {
        if self.bi_list.is_empty() {
            return Ok(false);
        }

        let last_bi = self.arena.get(self.bi_list[self.bi_list.len() - 1])
            .ok_or_else(|| ChanException::new(
                "Invalid bi index".to_string(),
                ErrCode::CommonError
            ))?;

        if !last_bi.is_sure {
            return Ok(false);
        }

        let last_end_klc = self.kline_arena.get(last_bi.end_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid kline index".to_string(),
                ErrCode::CommonError
            ))?;

        if last_end_klc.fx == FxType::Unknown {
            return Ok(false);
        }

        if self.can_make_bi(last_end_klc, last_klc)? {
            self.add_virtual_bi(last_end_klc.idx, last_klc.idx)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Add a virtual bi
    pub fn add_virtual_bi(&mut self, begin_klc_idx: Index, end_klc_idx: Index) -> PyResult<()> {
        let begin_klc = self.kline_arena.get(begin_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid begin kline index".to_string(),
                ErrCode::CommonError
            ))?;
        let end_klc = self.kline_arena.get(end_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid end kline index".to_string(),
                ErrCode::CommonError
            ))?;

        let bi = Bi::new(begin_klc, end_klc, self.bi_list.len(), false, &self.kline_arena)?;
        let bi_idx = self.arena.insert(bi);

        if let Some(last_bi_idx) = self.bi_list.last() {
            if let Some(last_bi) = self.arena.get_mut(*last_bi_idx) {
                last_bi.next = Some(bi_idx);
            }
            if let Some(new_bi) = self.arena.get_mut(bi_idx) {
                new_bi.pre = Some(*last_bi_idx);
            }
        }

        self.bi_list.push(bi_idx);
        Ok(())
    }

    /// Update bi sure status
    pub fn update_bi_sure(&mut self, klc: &KLine) -> PyResult<bool> {
        if self.bi_list.is_empty() {
            return Ok(false);
        }

        let last_bi_idx = self.bi_list[self.bi_list.len() - 1];
        let last_bi = self.arena.get(last_bi_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid bi index".to_string(),
                ErrCode::CommonError
            ))?;

        if last_bi.is_sure {
            return Ok(false);
        }

        let last_end_klc = self.kline_arena.get(last_bi.end_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid kline index".to_string(),
                ErrCode::CommonError
            ))?;

        if last_end_klc.fx == klc.fx {
            if self.can_update_peak(klc)? {
                if let Some(last_bi) = self.arena.get_mut(last_bi_idx) {
                    last_bi.set(
                        self.kline_arena.get(last_bi.begin_klc_idx).unwrap(),
                        klc,
                        &self.kline_arena
                    )?;
                }
                return Ok(true);
            }
            return Ok(false);
        }

        if self.can_make_bi(last_end_klc, klc)? {
            if let Some(last_bi) = self.arena.get_mut(last_bi_idx) {
                last_bi.is_sure = true;
                last_bi.sure_end.push(last_bi.end_klc_idx);
            }
            self.add_new_bi(last_end_klc.idx, klc.idx)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Check if can make bi
    pub fn can_make_bi(&self, begin_klc: &KLine, end_klc: &KLine) -> PyResult<bool> {
        if begin_klc.fx == FxType::Unknown || end_klc.fx == FxType::Unknown {
            return Ok(false);
        }

        if begin_klc.fx == end_klc.fx {
            return Ok(false);
        }

        let mut current = begin_klc;
        while current.idx < end_klc.idx {
            if let Some(next) = self.kline_arena.get(current.next.ok_or_else(|| 
                ChanException::new("Invalid next kline".to_string(), ErrCode::CommonError))?) {
                if next.fx != FxType::Unknown && next.fx != begin_klc.fx && next.fx != end_klc.fx {
                    return Ok(false);
                }
                current = next;
            } else {
                return Err(ChanException::new(
                    "Invalid kline sequence".to_string(),
                    ErrCode::CommonError
                ).into());
            }
        }

        match begin_klc.fx {
            FxType::Bottom => {
                if end_klc.high <= begin_klc.low {
                    return Ok(false);
                }
            },
            FxType::Top => {
                if end_klc.low >= begin_klc.high {
                    return Ok(false);
                }
            },
            _ => return Ok(false),
        }

        Ok(true)
    }

    /// Get bi direction at index
    pub fn get_bi_dir(&self, idx: usize) -> PyResult<BiDir> {
        if idx >= self.bi_list.len() {
            return Err(ChanException::new(
                format!("Invalid bi index: {}", idx),
                ErrCode::CommonError
            ).into());
        }

        let bi = self.arena.get(self.bi_list[idx])
            .ok_or_else(|| ChanException::new(
                "Invalid bi reference".to_string(),
                ErrCode::CommonError
            ))?;

        Ok(bi.dir)
    }

    /// Convert to DataFrame
    pub fn to_dataframe(&self, py: Python) -> PyResult<PyObject> {
        let pandas = py.import("pandas")?;
        let data: Vec<HashMap<String, PyObject>> = self.bi_list.iter()
            .filter_map(|&idx| self.arena.get(idx))
            .map(|bi| {
                let mut map = HashMap::new();
                map.insert("idx".to_string(), bi.idx.to_object(py));
                map.insert("dir".to_string(), bi.dir.to_object(py));
                map.insert("is_sure".to_string(), bi.is_sure.to_object(py));
                // ... 添加更多字段
                map
            })
            .collect();

        Ok(pandas.call_method1("DataFrame", (data,))?)
    }

    /// Get previous bi
    pub fn get_pre_bi(&self, bi_idx: Index) -> PyResult<Option<&Bi>> {
        if let Some(bi) = self.arena.get(bi_idx) {
            if let Some(pre_idx) = bi.pre {
                return Ok(self.arena.get(pre_idx));
            }
        }
        Ok(None)
    }

    /// Get next bi
    pub fn get_next_bi(&self, bi_idx: Index) -> PyResult<Option<&Bi>> {
        if let Some(bi) = self.arena.get(bi_idx) {
            if let Some(next_idx) = bi.next {
                return Ok(self.arena.get(next_idx));
            }
        }
        Ok(None)
    }

    /// Get bi at index
    pub fn get_bi(&self, idx: usize) -> PyResult<&Bi> {
        if idx >= self.bi_list.len() {
            return Err(ChanException::new(
                format!("Invalid bi index: {}", idx),
                ErrCode::CommonError
            ).into());
        }
        self.arena.get(self.bi_list[idx])
            .ok_or_else(|| ChanException::new(
                "Invalid bi reference".to_string(),
                ErrCode::CommonError
            ).into())
    }

    /// Get bi by index mutably
    pub fn get_bi_mut(&mut self, idx: usize) -> PyResult<&mut Bi> {
        if idx >= self.bi_list.len() {
            return Err(ChanException::new(
                format!("Invalid bi index: {}", idx),
                ErrCode::CommonError
            ).into());
        }
        self.arena.get_mut(self.bi_list[idx])
            .ok_or_else(|| ChanException::new(
                "Invalid bi reference".to_string(),
                ErrCode::CommonError
            ).into())
    }

    /// Calculate MACD metrics for bi
    pub fn cal_macd_metrics(&self, bi_idx: Index, algo: MacdAlgo) -> PyResult<(f64, f64, f64)> {
        let bi = self.arena.get(bi_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid bi index".to_string(),
                ErrCode::CommonError
            ))?;

        let mut klu_list = Vec::new();
        let mut current = bi.begin_klc_idx;
        
        while let Some(klc) = self.kline_arena.get(current) {
            klu_list.extend(klc.units.iter()
                .filter_map(|&idx| self.kline_arena.get(idx))
                .map(|klu| (klu.time, klu.close)));
            
            if current == bi.end_klc_idx {
                break;
            }
            
            if let Some(next_idx) = klc.next {
                current = next_idx;
            } else {
                break;
            }
        }

        get_macd_metrics(&klu_list, algo)
    }

    /// Check if bi is valid
    pub fn check_bi_valid(&self, bi_idx: Index) -> PyResult<bool> {
        let bi = self.arena.get(bi_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid bi index".to_string(),
                ErrCode::CommonError
            ))?;

        let begin_klc = self.kline_arena.get(bi.begin_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid begin kline".to_string(),
                ErrCode::CommonError
            ))?;

        let end_klc = self.kline_arena.get(bi.end_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid end kline".to_string(),
                ErrCode::CommonError
            ))?;

        self.can_make_bi(begin_klc, end_klc)
    }

    /// Get overlap ratio with another bi
    pub fn get_overlap_ratio(&self, bi1_idx: Index, bi2_idx: Index) -> PyResult<f64> {
        let bi1 = self.arena.get(bi1_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid bi1 index".to_string(),
                ErrCode::CommonError
            ))?;

        let bi2 = self.arena.get(bi2_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid bi2 index".to_string(),
                ErrCode::CommonError
            ))?;

        let (bi1_start, bi1_end) = (bi1.get_begin_val()?, bi1.get_end_val()?);
        let (bi2_start, bi2_end) = (bi2.get_begin_val()?, bi2.get_end_val()?);

        Ok(has_overlap(bi1_start, bi1_end, bi2_start, bi2_end))
    }

    /// Merge two bi
    pub fn merge_bi(&mut self, idx1: usize, idx2: usize) -> PyResult<()> {
        if idx1 >= self.bi_list.len() || idx2 >= self.bi_list.len() {
            return Err(ChanException::new(
                "Invalid bi indices".to_string(),
                ErrCode::CommonError
            ).into());
        }

        let bi1 = self.get_bi(idx1)?;
        let bi2 = self.get_bi(idx2)?;

        if bi1.dir != bi2.dir {
            return Err(ChanException::new(
                "Cannot merge bi with different directions".to_string(),
                ErrCode::BiErr
            ).into());
        }

        let begin_klc = self.kline_arena.get(bi1.begin_klc_idx).unwrap();
        let end_klc = self.kline_arena.get(bi2.end_klc_idx).unwrap();

        let new_bi = Bi::new(begin_klc, end_klc, idx1, true, &self.kline_arena)?;
        let new_bi_idx = self.arena.insert(new_bi);

        // Update connections
        if let Some(pre_bi) = self.get_pre_bi(self.bi_list[idx1])? {
            if let Some(pre_bi) = self.arena.get_mut(pre_bi.idx) {
                pre_bi.next = Some(new_bi_idx);
            }
        }

        if let Some(next_bi) = self.get_next_bi(self.bi_list[idx2])? {
            if let Some(next_bi) = self.arena.get_mut(next_bi.idx) {
                next_bi.pre = Some(new_bi_idx);
            }
        }

        // Remove old bis
        self.arena.remove(self.bi_list[idx1]);
        self.arena.remove(self.bi_list[idx2]);
        self.bi_list.splice(idx1..=idx2, vec![new_bi_idx]);

        Ok(())
    }

    /// Split bi at given kline
    pub fn split_bi(&mut self, bi_idx: usize, split_klc_idx: Index) -> PyResult<()> {
        let bi = self.get_bi(bi_idx)?;
        let split_klc = self.kline_arena.get(split_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid split kline index".to_string(),
                ErrCode::CommonError
            ))?;

        let begin_klc = self.kline_arena.get(bi.begin_klc_idx).unwrap();
        let end_klc = self.kline_arena.get(bi.end_klc_idx).unwrap();

        // Create two new bis
        let bi1 = Bi::new(begin_klc, split_klc, bi_idx, true, &self.kline_arena)?;
        let bi2 = Bi::new(split_klc, end_klc, bi_idx + 1, true, &self.kline_arena)?;

        let bi1_idx = self.arena.insert(bi1);
        let bi2_idx = self.arena.insert(bi2);

        // Update connections
        if let Some(pre_bi) = self.get_pre_bi(self.bi_list[bi_idx])? {
            if let Some(pre_bi) = self.arena.get_mut(pre_bi.idx) {
                pre_bi.next = Some(bi1_idx);
            }
        }

        if let Some(next_bi) = self.get_next_bi(self.bi_list[bi_idx])? {
            if let Some(next_bi) = self.arena.get_mut(next_bi.idx) {
                next_bi.pre = Some(bi2_idx);
            }
        }

        // Remove old bi and insert new ones
        self.arena.remove(self.bi_list[bi_idx]);
        self.bi_list.splice(bi_idx..=bi_idx, vec![bi1_idx, bi2_idx]);

        Ok(())
    }
}

/// Helper function to check if end is peak
fn end_is_peak(last_end: &KLine, cur_end: &KLine, arena: &Arena<KLine>) -> PyResult<bool> {
    match last_end.fx {
        FxType::Bottom => {
            let cmp_thred = cur_end.high;
            let mut klc = last_end.get_next(arena)?;
            while klc.idx < cur_end.idx {
                if klc.high > cmp_thred {
                    return Ok(false);
                }
                klc = klc.get_next(arena)?;
            }
            Ok(true)
        },
        FxType::Top => {
            let cmp_thred = cur_end.low;
            let mut klc = last_end.get_next(arena)?;
            while klc.idx < cur_end.idx {
                if klc.low < cmp_thred {
                    return Ok(false);
                }
                klc = klc.get_next(arena)?;
            }
            Ok(true)
        },
        _ => Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_config;

    #[test]
    fn test_bi_list_creation() {
        let config = create_test_config();
        let bi_list = BiList::new(config.bi_conf);
        assert_eq!(bi_list.bi_list.len(), 0);
        assert!(bi_list.last_end.is_none());
    }

    #[test]
    fn test_first_bi_creation() {
        let mut bi_list = BiList::new(create_test_config().bi_conf);
        
        // Create test K-lines
        let kl1 = KLine::new_test(1, 100.0, 90.0, FxType::Bottom);
        let kl2 = KLine::new_test(2, 120.0, 110.0, FxType::Top);
        
        let kl1_idx = bi_list.kline_arena.insert(kl1);
        let kl2_idx = bi_list.kline_arena.insert(kl2);
        
        if let Some(kl1) = bi_list.kline_arena.get(kl1_idx) {
            assert!(!bi_list.try_create_first_bi(kl1).unwrap());
            assert_eq!(bi_list.free_klc_lst.len(), 1);
        }
        
        if let Some(kl2) = bi_list.kline_arena.get(kl2_idx) {
            assert!(bi_list.try_create_first_bi(kl2).unwrap());
            assert_eq!(bi_list.bi_list.len(), 1);
        }
    }

    #[test]
    fn test_bi_merge() {
        let mut bi_list = BiList::new(create_test_config().bi_conf);
        
        // Create test K-lines and bis
        let kl1 = KLine::new_test(1, 100.0, 90.0, FxType::Bottom);
        let kl2 = KLine::new_test(2, 120.0, 110.0, FxType::Top);
        let kl3 = KLine::new_test(3, 130.0, 120.0, FxType::Top);
        
        let kl1_idx = bi_list.kline_arena.insert(kl1);
        let kl2_idx = bi_list.kline_arena.insert(kl2);
        let kl3_idx = bi_list.kline_arena.insert(kl3);
        
        bi_list.add_new_bi(kl1_idx, kl2_idx).unwrap();
        bi_list.add_new_bi(kl2_idx, kl3_idx).unwrap();
        
        bi_list.merge_bi(0, 1).unwrap();
        assert_eq!(bi_list.bi_list.len(), 1);
    }

    #[test]
    fn test_bi_split() {
        let mut bi_list = BiList::new(create_test_config().bi_conf);
        
        // Create test K-lines and bi
        let kl1 = KLine::new_test(1, 100.0, 90.0, FxType::Bottom);
        let kl2 = KLine::new_test(2, 110.0, 100.0, FxType::Unknown);
        let kl3 = KLine::new_test(3, 120.0, 110.0, FxType::Top);
        
        let kl1_idx = bi_list.kline_arena.insert(kl1);
        let kl2_idx = bi_list.kline_arena.insert(kl2);
        let kl3_idx = bi_list.kline_arena.insert(kl3);
        
        bi_list.add_new_bi(kl1_idx, kl3_idx).unwrap();
        bi_list.split_bi(0, kl2_idx).unwrap();
        
        assert_eq!(bi_list.bi_list.len(), 2);
    }
} 