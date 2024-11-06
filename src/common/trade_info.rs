use pyo3::prelude::*;
use std::collections::HashMap;
use crate::common::enums::DataField;
use crate::math::{MacdItem, BollMetric};

/// Trade information for a K-line unit
#[pyclass]
#[derive(Debug, Clone)]
pub struct TradeInfo {
    pub volume: f64,      // 成交量
    pub turnover: f64,    // 成交额
    pub turnrate: f64,    // 换手率
    pub macd: Option<MacdItem>,  // MACD 指标
    pub boll: Option<BollMetric>, // 布林带指标
    pub kdj: Option<(f64, f64, f64)>, // KDJ 指标 (K, D, J)
    pub rsi: Option<f64>, // RSI 指标
}

#[pymethods]
impl TradeInfo {
    /// Create a new TradeInfo instance from a dictionary
    #[new]
    pub fn new(kl_dict: HashMap<DataField, PyObject>) -> PyResult<Self> {
        let volume = kl_dict.get(&DataField::Volume)
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(0.0);

        let turnover = kl_dict.get(&DataField::Turnover)
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(0.0);

        let turnrate = kl_dict.get(&DataField::TurnRate)
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(0.0);

        Ok(Self {
            volume,
            turnover,
            turnrate,
            macd: None,
            boll: None,
            kdj: None,
            rsi: None,
        })
    }

    /// Get the volume value
    #[getter]
    pub fn get_volume(&self) -> f64 {
        self.volume
    }

    /// Get the turnover value
    #[getter]
    pub fn get_turnover(&self) -> f64 {
        self.turnover
    }

    /// Get the turnrate value
    #[getter]
    pub fn get_turnrate(&self) -> f64 {
        self.turnrate
    }

    /// Get the MACD indicator
    #[getter]
    pub fn get_macd(&self) -> Option<MacdItem> {
        self.macd.clone()
    }

    /// Get the Bollinger Bands indicator
    #[getter]
    pub fn get_boll(&self) -> Option<BollMetric> {
        self.boll.clone()
    }

    /// Get the KDJ indicator
    #[getter]
    pub fn get_kdj(&self) -> Option<(f64, f64, f64)> {
        self.kdj
    }

    /// Get the RSI indicator
    #[getter]
    pub fn get_rsi(&self) -> Option<f64> {
        self.rsi
    }

    /// Check if the trade info contains too many zero values
    pub fn has_too_much_zero(&self) -> bool {
        let mut zero_count = 0;
        let total_count = 3; // volume, turnover, turnrate

        if self.volume == 0.0 { zero_count += 1; }
        if self.turnover == 0.0 { zero_count += 1; }
        if self.turnrate == 0.0 { zero_count += 1; }

        zero_count > total_count / 2
    }

    /// Convert to string representation
    fn __str__(&self) -> String {
        format!("TradeInfo(volume={}, turnover={}, turnrate={})",
            self.volume, self.turnover, self.turnrate)
    }
}

impl TradeInfo {
    /// Set the MACD indicator
    pub fn set_macd(&mut self, macd: MacdItem) {
        self.macd = Some(macd);
    }

    /// Set the Bollinger Bands indicator
    pub fn set_boll(&mut self, boll: BollMetric) {
        self.boll = Some(boll);
    }

    /// Set the KDJ indicator
    pub fn set_kdj(&mut self, k: f64, d: f64, j: f64) {
        self.kdj = Some((k, d, j));
    }

    /// Set the RSI indicator
    pub fn set_rsi(&mut self, rsi: f64) {
        self.rsi = Some(rsi);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_info_creation() {
        let mut kl_dict = HashMap::new();
        let gil = Python::acquire_gil();
        let py = gil.python();
        
        kl_dict.insert(DataField::Volume, 1000.0.into_py(py));
        kl_dict.insert(DataField::Turnover, 50000.0.into_py(py));
        kl_dict.insert(DataField::TurnRate, 0.15.into_py(py));

        let trade_info = TradeInfo::new(kl_dict).unwrap();
        assert_eq!(trade_info.volume, 1000.0);
        assert_eq!(trade_info.turnover, 50000.0);
        assert_eq!(trade_info.turnrate, 0.15);
    }

    #[test]
    fn test_has_too_much_zero() {
        let mut kl_dict = HashMap::new();
        let gil = Python::acquire_gil();
        let py = gil.python();
        
        kl_dict.insert(DataField::Volume, 0.0.into_py(py));
        kl_dict.insert(DataField::Turnover, 0.0.into_py(py));
        kl_dict.insert(DataField::TurnRate, 0.15.into_py(py));

        let trade_info = TradeInfo::new(kl_dict).unwrap();
        assert!(trade_info.has_too_much_zero());
    }

    #[test]
    fn test_indicators() {
        let mut trade_info = TradeInfo::new(HashMap::new()).unwrap();
        
        // Test MACD
        let macd = MacdItem::new(1.0, 2.0, 3.0);
        trade_info.set_macd(macd.clone());
        assert_eq!(trade_info.get_macd().unwrap(), macd);

        // Test Bollinger Bands
        let boll = BollMetric::new(10.0, 11.0, 9.0);
        trade_info.set_boll(boll.clone());
        assert_eq!(trade_info.get_boll().unwrap(), boll);

        // Test KDJ
        trade_info.set_kdj(50.0, 55.0, 45.0);
        assert_eq!(trade_info.get_kdj().unwrap(), (50.0, 55.0, 45.0));

        // Test RSI
        trade_info.set_rsi(65.0);
        assert_eq!(trade_info.get_rsi().unwrap(), 65.0);
    }
} 