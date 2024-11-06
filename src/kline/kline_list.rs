use pyo3::prelude::*;
use generational_arena::{Arena, Index};
use std::collections::HashMap;
use crate::common::enums::{KLineType, KLineDir, SegType};
use crate::common::error::{ChanException, ErrCode};
use crate::bi::bi_list::BiList;
use crate::seg::{SegConfig, SegListComm};
use crate::zs::zs_list::ZSList;
use crate::bs_point::bs_point_list::BSPointList;
use crate::chan_config::ChanConfig;
use crate::kline::{KLine, KLineUnit};

/// Manages a list of K-lines and their analysis
#[pyclass]
#[derive(Debug)]
pub struct KLineList {
    pub kl_type: KLineType,                      // K线类型
    pub config: ChanConfig,                      // 配置信息
    pub klines: Vec<Index>,                      // K线索引列表
    pub bi_list: BiList,                         // 笔列表
    pub seg_list: SegListComm,                   // 线段列表
    pub segseg_list: SegListComm,                // 线段的线段列表
    pub zs_list: ZSList,                         // 中枢列表
    pub segzs_list: ZSList,                      // 线段中枢列表
    pub bs_point_lst: BSPointList,               // 买卖点列表
    pub seg_bs_point_lst: BSPointList,           // 线段买卖点列表
    pub metric_model_lst: Vec<PyObject>,         // 度量模型列表
    pub step_calculation: bool,                  // 是否需要逐步计算
    pub bs_point_history: Vec<HashMap<String, PyObject>>,    // 买卖点历史
    pub seg_bs_point_history: Vec<HashMap<String, PyObject>>, // 线段买卖点历史
    arena: Arena<KLine>,                         // K线对象管理
    unit_arena: Arena<KLineUnit>,                // K线单元对象管理
}

#[pymethods]
impl KLineList {
    /// Create a new KLineList instance
    #[new]
    pub fn new(kl_type: KLineType, conf: ChanConfig) -> PyResult<Self> {
        let seg_list = get_seglist_instance(&conf.seg_conf, SegType::Bi)?;
        let segseg_list = get_seglist_instance(&conf.seg_conf, SegType::Seg)?;

        Ok(Self {
            kl_type,
            config: conf.clone(),
            klines: Vec::new(),
            bi_list: BiList::new(conf.bi_conf),
            seg_list,
            segseg_list,
            zs_list: ZSList::new(conf.zs_conf),
            segzs_list: ZSList::new(conf.zs_conf),
            bs_point_lst: BSPointList::new(conf.bs_point_conf),
            seg_bs_point_lst: BSPointList::new(conf.seg_bs_point_conf),
            metric_model_lst: conf.get_metric_model(),
            step_calculation: false,
            bs_point_history: Vec::new(),
            seg_bs_point_history: Vec::new(),
            arena: Arena::new(),
            unit_arena: Arena::new(),
        })
    }

    /// Get length of K-line list
    fn __len__(&self) -> usize {
        self.klines.len()
    }

    /// Get K-line by index
    fn __getitem__(&self, index: PyObject, py: Python) -> PyResult<PyObject> {
        if let Ok(idx) = index.extract::<usize>(py) {
            if idx >= self.klines.len() {
                return Err(PyIndexError::new_err("Index out of range"));
            }
            if let Some(kline) = self.arena.get(self.klines[idx]) {
                return Ok(kline.to_object(py));
            }
        } else if let Ok(slice) = index.extract::<PySlice>(py) {
            let indices = slice.indices(self.klines.len() as i64)?;
            let result: Vec<_> = (indices.start..indices.stop)
                .step_by(indices.step as usize)
                .filter_map(|i| self.arena.get(self.klines[i as usize]))
                .map(|kl| kl.to_object(py))
                .collect();
            return Ok(result.to_object(py));
        }
        Err(PyTypeError::new_err("Invalid index type"))
    }

    /// Calculate segments and ZS (中枢)
    pub fn cal_seg_and_zs(&mut self) -> PyResult<()> {
        if !self.step_calculation {
            self.bi_list.try_add_virtual_bi(
                self.arena.get(self.klines.last().ok_or_else(|| 
                    ChanException::new("Empty kline list".to_string(), ErrCode::CommonError))?)?
            )?;
        }

        cal_seg(&mut self.bi_list, &mut self.seg_list)?;
        self.zs_list.cal_bi_zs(&self.bi_list, &self.seg_list)?;
        update_zs_in_seg(&mut self.bi_list, &mut self.seg_list, &mut self.zs_list)?;

        cal_seg(&mut self.seg_list, &mut self.segseg_list)?;
        self.segzs_list.cal_bi_zs(&self.seg_list, &self.segseg_list)?;
        update_zs_in_seg(&mut self.seg_list, &mut self.segseg_list, &mut self.segzs_list)?;

        // Calculate buy/sell points
        self.seg_bs_point_lst.cal(&self.seg_list, &self.segseg_list)?;
        self.bs_point_lst.cal(&self.bi_list, &self.seg_list)?;
        self.record_current_bs_points()?;

        Ok(())
    }

    /// Add a single K-line unit
    pub fn add_single_klu(&mut self, klu: KLineUnit) -> PyResult<()> {
        let klu_idx = self.unit_arena.insert(klu);
        if let Some(klu) = self.unit_arena.get_mut(klu_idx) {
            klu.set_metric(&self.metric_model_lst)?;
        }

        if self.klines.is_empty() {
            let kline = KLine::new(
                self.unit_arena.get(klu_idx).ok_or_else(|| 
                    ChanException::new("Invalid KLineUnit".to_string(), ErrCode::CommonError))?,
                0,
                KLineDir::Up
            )?;
            let kl_idx = self.arena.insert(kline);
            self.klines.push(kl_idx);
        } else {
            let last_kl_idx = *self.klines.last().unwrap();
            let dir = if let Some(last_kl) = self.arena.get(last_kl_idx) {
                last_kl.try_add(klu_idx, &self.unit_arena)?
            } else {
                return Err(ChanException::new("Invalid KLine".to_string(), ErrCode::CommonError).into());
            };

            if dir != KLineDir::Combine {
                let new_kline = KLine::new(
                    self.unit_arena.get(klu_idx).ok_or_else(|| 
                        ChanException::new("Invalid KLineUnit".to_string(), ErrCode::CommonError))?,
                    self.klines.len(),
                    dir
                )?;
                let new_kl_idx = self.arena.insert(new_kline);
                
                if let Some(last_kl) = self.arena.get_mut(last_kl_idx) {
                    last_kl.set_next(Some(new_kl_idx));
                }
                if let Some(new_kl) = self.arena.get_mut(new_kl_idx) {
                    new_kl.set_pre(Some(last_kl_idx));
                }
                
                self.klines.push(new_kl_idx);

                if self.klines.len() >= 3 {
                    let last_three = self.get_last_three_klines()?;
                    last_three.1.update_fx(last_three.0, last_three.2)?;
                }

                if self.bi_list.update_bi(
                    self.arena.get(self.klines[self.klines.len() - 2]).unwrap(),
                    self.arena.get(self.klines.last().unwrap()).unwrap(),
                    self.step_calculation
                )? && self.step_calculation {
                    self.cal_seg_and_zs()?;
                }
            } else if self.step_calculation && self.bi_list.try_add_virtual_bi(
                self.arena.get(self.klines.last().unwrap()).unwrap(),
                true
            )? {
                self.cal_seg_and_zs()?;
            }
        }
        Ok(())
    }

    /// Iterate over K-line units
    pub fn klu_iter(&self, klc_begin_idx: usize) -> impl Iterator<Item = &KLineUnit> {
        self.klines[klc_begin_idx..].iter()
            .filter_map(|&kl_idx| self.arena.get(kl_idx))
            .flat_map(|kl| kl.units.iter())
            .filter_map(move |&unit_idx| self.unit_arena.get(unit_idx))
    }

    /// Convert to DataFrames
    pub fn to_dataframes(&self, py: Python) -> PyResult<HashMap<String, PyObject>> {
        let mut dataframes = HashMap::new();

        // Convert kline_list to DataFrame
        let kline_data: Vec<HashMap<String, PyObject>> = self.klines.iter()
            .filter_map(|&idx| self.arena.get(idx))
            .map(|kl| {
                let mut map = HashMap::new();
                map.insert("begin_time".to_string(), kl.time_begin.to_object(py));
                map.insert("end_time".to_string(), kl.time_end.to_object(py));
                map.insert("idx".to_string(), kl.idx.to_object(py));
                map.insert("dir".to_string(), kl.dir.to_object(py));
                map.insert("high".to_string(), kl.high.to_object(py));
                map.insert("low".to_string(), kl.low.to_object(py));
                map.insert("fx".to_string(), kl.fx.to_object(py));
                map
            })
            .collect();
        dataframes.insert("kline_list".to_string(), 
            py.import("pandas")?.call_method1("DataFrame", (kline_data,))?);

        // Convert bi_list to DataFrame
        let bi_data = self.bi_list.to_dataframe(py)?;
        dataframes.insert("bi_list".to_string(), bi_data);

        // Convert seg_list to DataFrame
        let seg_data = self.seg_list.to_dataframe(py)?;
        dataframes.insert("seg_list".to_string(), seg_data);

        // Convert segseg_list to DataFrame
        let segseg_data = self.segseg_list.to_dataframe(py)?;
        dataframes.insert("segseg_list".to_string(), segseg_data);

        // Convert zs_list to DataFrame
        let zs_data = self.zs_list.to_dataframe(py)?;
        dataframes.insert("zs_list".to_string(), zs_data);

        // Convert segzs_list to DataFrame
        let segzs_data = self.segzs_list.to_dataframe(py)?;
        dataframes.insert("segzs_list".to_string(), segzs_data);

        // Convert bs_point_lst to DataFrame
        let bs_point_data = self.bs_point_lst.to_dataframe(py)?;
        dataframes.insert("bs_point_lst".to_string(), bs_point_data);

        // Convert seg_bs_point_lst to DataFrame
        let seg_bs_point_data = self.seg_bs_point_lst.to_dataframe(py)?;
        dataframes.insert("seg_bs_point_lst".to_string(), seg_bs_point_data);

        // Add historical bs_points
        dataframes.insert("bs_point_history".to_string(), 
            py.import("pandas")?.call_method1("DataFrame", (self.bs_point_history.clone(),))?);

        // Add historical seg_bs_points
        dataframes.insert("seg_bs_point_history".to_string(), 
            py.import("pandas")?.call_method1("DataFrame", (self.seg_bs_point_history.clone(),))?);

        Ok(dataframes)
    }
}

/// Get seglist instance based on configuration
fn get_seglist_instance(seg_config: &SegConfig, lv: SegType) -> PyResult<SegListComm> {
    match seg_config.seg_algo.as_str() {
        "chan" => {
            Ok(SegListComm::Chan(seg_config.clone(), lv))
        },
        "1+1" => {
            println!("Please avoid using seg_algo={} as it is deprecated and no longer maintained.", 
                seg_config.seg_algo);
            Ok(SegListComm::Dyh(seg_config.clone(), lv))
        },
        "break" => {
            println!("Please avoid using seg_algo={} as it is deprecated and no longer maintained.", 
                seg_config.seg_algo);
            Ok(SegListComm::Def(seg_config.clone(), lv))
        },
        _ => Err(ChanException::new(
            format!("unsupport seg algorithm:{}", seg_config.seg_algo),
            ErrCode::ParaError
        ).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_config;

    #[test]
    fn test_kline_list_creation() {
        let config = create_test_config();
        let kl_list = KLineList::new(KLineType::KDay, config).unwrap();
        assert_eq!(kl_list.klines.len(), 0);
    }

    #[test]
    fn test_add_kline_unit() {
        let config = create_test_config();
        let mut kl_list = KLineList::new(KLineType::KDay, config).unwrap();
        
        let klu = KLineUnit::new_test(1234567890, 100.0, 105.0, 110.0, 95.0);
        kl_list.add_single_klu(klu).unwrap();
        
        assert_eq!(kl_list.klines.len(), 1);
        assert_eq!(kl_list.arena.get(kl_list.klines[0]).unwrap().high, 110.0);
    }

    // Add more tests...
} 