use pyo3::prelude::*;
use generational_arena::{Arena, Index};
use std::collections::HashMap;
use crate::common::enums::{BiDir, BiType, DataField, FxType, MacdAlgo};
use crate::common::error::{ChanException, ErrCode};
use crate::kline::{KLine, KLineUnit};
use crate::bs_point::bs_point::BSPoint;
use crate::seg::seg::Seg;
use crate::common::cache::make_cache;

/// Represents a Bi (笔) in the Chan system
#[pyclass]
#[derive(Debug)]
pub struct Bi {
    pub begin_klc_idx: Index,        // 起始K线组合索引
    pub end_klc_idx: Index,          // 结束K线组合索引
    pub dir: BiDir,                  // 方向
    pub idx: usize,                  // 索引
    pub bi_type: BiType,             // 笔类型
    pub is_sure: bool,               // 是否确定
    pub sure_end: Vec<Index>,        // 确定的结束点列表
    pub seg_idx: Option<usize>,      // 所属线段索引
    pub parent_seg: Option<Index>,   // 父线段索引
    pub bsp: Option<BSPoint>,        // 买卖点
    pub next: Option<Index>,         // 下一笔索引
    pub pre: Option<Index>,          // 前一笔索引
    pub cache: HashMap<String, PyObject>, // 缓存
}

#[pymethods]
impl Bi {
    /// Create a new Bi instance
    #[new]
    pub fn new(
        begin_klc: &KLine, 
        end_klc: &KLine, 
        idx: usize, 
        is_sure: bool,
        arena: &Arena<KLine>
    ) -> PyResult<Self> {
        let mut bi = Self {
            begin_klc_idx: Index::from_raw_parts(0, 0), // 临时值
            end_klc_idx: Index::from_raw_parts(0, 0),   // 临时值
            dir: BiDir::Up,  // 临时值
            idx,
            bi_type: BiType::Strict,
            is_sure,
            sure_end: Vec::new(),
            seg_idx: None,
            parent_seg: None,
            bsp: None,
            next: None,
            pre: None,
            cache: HashMap::new(),
        };
        bi.set(begin_klc, end_klc, arena)?;
        Ok(bi)
    }

    /// Clean the cache
    pub fn clean_cache(&mut self) {
        self.cache.clear();
    }

    /// Get begin K-line combination
    #[getter]
    pub fn begin_klc<'a>(&self, arena: &'a Arena<KLine>) -> PyResult<&'a KLine> {
        arena.get(self.begin_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid begin_klc_idx".to_string(), 
                ErrCode::CommonError
            ).into())
    }

    /// Get end K-line combination
    #[getter]
    pub fn end_klc<'a>(&self, arena: &'a Arena<KLine>) -> PyResult<&'a KLine> {
        arena.get(self.end_klc_idx)
            .ok_or_else(|| ChanException::new(
                "Invalid end_klc_idx".to_string(), 
                ErrCode::CommonError
            ).into())
    }

    /// Get direction
    #[getter]
    pub fn dir(&self) -> BiDir {
        self.dir
    }

    /// Get index
    #[getter]
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Get type
    #[getter]
    pub fn bi_type(&self) -> BiType {
        self.bi_type
    }

    /// Get sure status
    #[getter]
    pub fn is_sure(&self) -> bool {
        self.is_sure
    }

    /// Get sure end list
    #[getter]
    pub fn sure_end(&self) -> Vec<Index> {
        self.sure_end.clone()
    }

    /// Get K-line list
    pub fn klc_lst<'a>(&self, arena: &'a Arena<KLine>) -> impl Iterator<Item = &'a KLine> {
        let mut current_idx = Some(self.begin_klc_idx);
        std::iter::from_fn(move || {
            let idx = current_idx?;
            let klc = arena.get(idx)?;
            if idx == self.end_klc_idx {
                current_idx = None;
            } else {
                current_idx = klc.next;
            }
            Some(klc)
        })
    }

    /// Get K-line list in reverse
    pub fn klc_lst_re<'a>(&self, arena: &'a Arena<KLine>) -> impl Iterator<Item = &'a KLine> {
        let mut current_idx = Some(self.end_klc_idx);
        std::iter::from_fn(move || {
            let idx = current_idx?;
            let klc = arena.get(idx)?;
            if idx == self.begin_klc_idx {
                current_idx = None;
            } else {
                current_idx = klc.pre;
            }
            Some(klc)
        })
    }

    /// Get segment index
    #[getter]
    pub fn seg_idx(&self) -> Option<usize> {
        self.seg_idx
    }

    /// Set segment index
    pub fn set_seg_idx(&mut self, idx: usize) {
        self.seg_idx = Some(idx);
    }

    /// String representation
    fn __str__(&self, arena: &Arena<KLine>) -> PyResult<String> {
        Ok(format!("{}|{} ~ {}", 
            self.dir,
            self.begin_klc(arena)?,
            self.end_klc(arena)?))
    }

    /// Check validity
    pub fn check(&self, arena: &Arena<KLine>) -> PyResult<()> {
        let begin_klc = self.begin_klc(arena)?;
        let end_klc = self.end_klc(arena)?;

        match self.dir {
            BiDir::Down => {
                if begin_klc.high <= end_klc.low {
                    return Err(ChanException::new(
                        format!("{}:{}~{} 笔的方向和收尾位置不一致!", 
                            self.idx,
                            begin_klc.time_begin,
                            end_klc.time_end),
                        ErrCode::BiErr
                    ).into());
                }
            },
            BiDir::Up => {
                if begin_klc.low >= end_klc.high {
                    return Err(ChanException::new(
                        format!("{}:{}~{} 笔的方向和收尾位置不一致!", 
                            self.idx,
                            begin_klc.time_begin,
                            end_klc.time_end),
                        ErrCode::BiErr
                    ).into());
                }
            }
        }
        Ok(())
    }

    /// Set begin and end K-lines
    pub fn set(&mut self, begin_klc: &KLine, end_klc: &KLine, arena: &Arena<KLine>) -> PyResult<()> {
        self.begin_klc_idx = Index::from_raw_parts(begin_klc.idx, 1);
        self.end_klc_idx = Index::from_raw_parts(end_klc.idx, 1);

        self.dir = match begin_klc.fx {
            FxType::Bottom => BiDir::Up,
            FxType::Top => BiDir::Down,
            _ => return Err(ChanException::new(
                "ERROR DIRECTION when creating bi".to_string(),
                ErrCode::BiErr
            ).into())
        };

        self.check(arena)?;
        self.clean_cache();
        Ok(())
    }

    /// Get begin value
    #[make_cache]
    pub fn get_begin_val(&self, arena: &Arena<KLine>) -> PyResult<f64> {
        let begin_klc = self.begin_klc(arena)?;
        Ok(if self.is_up() {
            begin_klc.low
        } else {
            begin_klc.high
        })
    }

    /// Get end value
    #[make_cache]
    pub fn get_end_val(&self, arena: &Arena<KLine>) -> PyResult<f64> {
        let end_klc = self.end_klc(arena)?;
        Ok(if self.is_up() {
            end_klc.high
        } else {
            end_klc.low
        })
    }

    // ... 更多方法实现 ...
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_kline;

    #[test]
    fn test_bi_creation() {
        let mut arena = Arena::new();
        
        let begin_kl = create_test_kline(1, 100.0, 90.0, FxType::Bottom);
        let end_kl = create_test_kline(2, 120.0, 110.0, FxType::Top);
        
        let begin_idx = arena.insert(begin_kl);
        let end_idx = arena.insert(end_kl);
        
        if let Some(begin_kl) = arena.get(begin_idx) {
            if let Some(end_kl) = arena.get(end_idx) {
                let bi = Bi::new(begin_kl, end_kl, 0, true, &arena).unwrap();
                
                assert_eq!(bi.dir, BiDir::Up);
                assert_eq!(bi.idx, 0);
                assert!(bi.is_sure);
                assert!(bi.sure_end.is_empty());
            }
        }
    }

    #[test]
    fn test_bi_direction() {
        let mut arena = Arena::new();
        
        let begin_kl = create_test_kline(1, 120.0, 110.0, FxType::Top);
        let end_kl = create_test_kline(2, 100.0, 90.0, FxType::Bottom);
        
        let begin_idx = arena.insert(begin_kl);
        let end_idx = arena.insert(end_kl);
        
        if let Some(begin_kl) = arena.get(begin_idx) {
            if let Some(end_kl) = arena.get(end_idx) {
                let bi = Bi::new(begin_kl, end_kl, 0, true, &arena).unwrap();
                
                assert_eq!(bi.dir, BiDir::Down);
            }
        }
    }
} 