use pyo3::prelude::*;
use pyo3::exceptions::PyException;
use std::fmt;

/// Error codes for the Chan system
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrCode {
    // Chan errors (0-99)
    #[pyo3(name = "_CHAN_ERR_BEGIN")]
    ChanErrBegin = 0,
    #[pyo3(name = "COMMON_ERROR")]
    CommonError = 1,
    #[pyo3(name = "SRC_DATA_NOT_FOUND")]
    SrcDataNotFound = 3,
    #[pyo3(name = "SRC_DATA_TYPE_ERR")]
    SrcDataTypeErr = 4,
    #[pyo3(name = "PARA_ERROR")]
    ParaError = 5,
    #[pyo3(name = "EXTRA_KLU_ERR")]
    ExtraKluErr = 6,
    #[pyo3(name = "SEG_END_VALUE_ERR")]
    SegEndValueErr = 7,
    #[pyo3(name = "SEG_EIGEN_ERR")]
    SegEigenErr = 8,
    #[pyo3(name = "BI_ERR")]
    BiErr = 9,
    #[pyo3(name = "COMBINER_ERR")]
    CombinerErr = 10,
    #[pyo3(name = "PLOT_ERR")]
    PlotErr = 11,
    #[pyo3(name = "MODEL_ERROR")]
    ModelError = 12,
    #[pyo3(name = "SEG_LEN_ERR")]
    SegLenErr = 13,
    #[pyo3(name = "ENV_CONF_ERR")]
    EnvConfErr = 14,
    #[pyo3(name = "UNKNOWN_DB_TYPE")]
    UnknownDbType = 15,
    #[pyo3(name = "FEATURE_ERROR")]
    FeatureError = 16,
    #[pyo3(name = "CONFIG_ERROR")]
    ConfigError = 17,
    #[pyo3(name = "SRC_DATA_FORMAT_ERROR")]
    SrcDataFormatError = 18,
    #[pyo3(name = "_CHAN_ERR_END")]
    ChanErrEnd = 99,

    // Trade errors (100-199)
    #[pyo3(name = "_TRADE_ERR_BEGIN")]
    TradeErrBegin = 100,
    #[pyo3(name = "SIGNAL_EXISTED")]
    SignalExisted = 101,
    #[pyo3(name = "RECORD_NOT_EXIST")]
    RecordNotExist = 102,
    #[pyo3(name = "RECORD_ALREADY_OPENED")]
    RecordAlreadyOpened = 103,
    #[pyo3(name = "QUOTA_NOT_ENOUGH")]
    QuotaNotEnough = 104,
    #[pyo3(name = "RECORD_NOT_OPENED")]
    RecordNotOpened = 105,
    #[pyo3(name = "TRADE_UNLOCK_FAIL")]
    TradeUnlockFail = 106,
    #[pyo3(name = "PLACE_ORDER_FAIL")]
    PlaceOrderFail = 107,
    #[pyo3(name = "LIST_ORDER_FAIL")]
    ListOrderFail = 108,
    #[pyo3(name = "CANDEL_ORDER_FAIL")]
    CancelOrderFail = 109,
    #[pyo3(name = "GET_FUTU_PRICE_FAIL")]
    GetFutuPriceFail = 110,
    #[pyo3(name = "GET_FUTU_LOT_SIZE_FAIL")]
    GetFutuLotSizeFail = 111,
    #[pyo3(name = "OPEN_RECORD_NOT_WATCHING")]
    OpenRecordNotWatching = 112,
    #[pyo3(name = "GET_HOLDING_QTY_FAIL")]
    GetHoldingQtyFail = 113,
    #[pyo3(name = "RECORD_CLOSED")]
    RecordClosed = 114,
    #[pyo3(name = "REQUEST_TRADING_DAYS_FAIL")]
    RequestTradingDaysFail = 115,
    #[pyo3(name = "COVER_ORDER_ID_NOT_UNIQUE")]
    CoverOrderIdNotUnique = 116,
    #[pyo3(name = "SIGNAL_TRADED")]
    SignalTraded = 117,
    #[pyo3(name = "_TRADE_ERR_END")]
    TradeErrEnd = 199,

    // KL data errors (200-299)
    #[pyo3(name = "_KL_ERR_BEGIN")]
    KlErrBegin = 200,
    #[pyo3(name = "PRICE_BELOW_ZERO")]
    PriceBelowZero = 201,
    #[pyo3(name = "KL_DATA_NOT_ALIGN")]
    KlDataNotAlign = 202,
    #[pyo3(name = "KL_DATA_INVALID")]
    KlDataInvalid = 203,
    #[pyo3(name = "KL_TIME_INCONSISTENT")]
    KlTimeInconsistent = 204,
    #[pyo3(name = "TRADEINFO_TOO_MUCH_ZERO")]
    TradeinfoTooMuchZero = 205,
    #[pyo3(name = "KL_NOT_MONOTONOUS")]
    KlNotMonotonous = 206,
    #[pyo3(name = "SNAPSHOT_ERR")]
    SnapshotErr = 207,
    #[pyo3(name = "SUSPENSION")]
    Suspension = 208,  // 疑似停牌
    #[pyo3(name = "STOCK_IPO_TOO_LATE")]
    StockIpoTooLate = 209,
    #[pyo3(name = "NO_DATA")]
    NoData = 210,
    #[pyo3(name = "STOCK_NOT_ACTIVE")]
    StockNotActive = 211,
    #[pyo3(name = "STOCK_PRICE_NOT_ACTIVE")]
    StockPriceNotActive = 212,
    #[pyo3(name = "_KL_ERR_END")]
    KlErrEnd = 299,
}

/// Chan system exception
#[pyclass(extends=PyException)]
#[derive(Debug)]
pub struct ChanException {
    pub errcode: ErrCode,
    pub msg: String,
}

#[pymethods]
impl ChanException {
    #[new]
    pub fn new(message: String, code: ErrCode) -> Self {
        Self {
            errcode: code,
            msg: message,
        }
    }

    /// Check if the error is a KL data error
    #[pyo3(name = "is_kldata_err")]
    pub fn is_kldata_err(&self) -> bool {
        (self.errcode as i32) > (ErrCode::KlErrBegin as i32) 
            && (self.errcode as i32) < (ErrCode::KlErrEnd as i32)
    }

    /// Check if the error is a Chan error
    #[pyo3(name = "is_chan_err")]
    pub fn is_chan_err(&self) -> bool {
        (self.errcode as i32) > (ErrCode::ChanErrBegin as i32) 
            && (self.errcode as i32) < (ErrCode::ChanErrEnd as i32)
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(self.to_string())
    }
}

impl fmt::Display for ChanException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.errcode as i32, self.msg)
    }
}

impl std::error::Error for ChanException {}

// Python module initialization
#[pymodule]
fn chan_error(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<ErrCode>()?;
    m.add_class::<ChanException>()?;
    Ok(())
}

// Example usage in Rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chan_exception() {
        let exc = ChanException::new(
            "Configuration error".to_string(),
            ErrCode::ConfigError
        );
        assert!(exc.is_chan_err());
        assert!(!exc.is_kldata_err());
        assert_eq!(exc.errcode, ErrCode::ConfigError);
    }
} 