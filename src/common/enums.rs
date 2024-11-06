use pyo3::prelude::*;
use std::str::FromStr;

/// Data source types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataSource {
    BaoStock,
    Ccxt,
    Csv,
}

/// K-line time period types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KLineType {
    K1S = 1,
    K3S = 2,
    K5S = 3,
    K10S = 4,
    K15S = 5,
    K20S = 6,
    K30S = 7,
    K1M = 8,
    K3M = 9,
    K5M = 10,
    K10M = 11,
    K15M = 12,
    K30M = 13,
    K60M = 14,
    KDay = 15,
    KWeek = 16,
    KMonth = 17,
    KQuarter = 18,
    KYear = 19,
}

/// K-line direction types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KLineDir {
    Up,
    Down,
    Combine,
    Included,
}

/// FX (Fractal) types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxType {
    Bottom,
    Top,
    Unknown,
}

/// BI (笔) direction types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiDir {
    Up,
    Down,
}

/// BI (笔) types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiType {
    Unknown,
    Strict,
    SubValue,    // 次高低点成笔
    TiaokongThred,
    Daheng,
    Tuibi,
    Unstrict,
    TiaokongValue,
}

/// BSP types with main type support
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BspType {
    T1,
    T1P,
    T2,
    T2S,
    T3A,    // 中枢在1类后面
    T3B,    // 中枢在1类前面
}

impl BspType {
    pub fn main_type(&self) -> &str {
        match self {
            BspType::T1 | BspType::T1P => "1",
            BspType::T2 | BspType::T2S => "2",
            BspType::T3A | BspType::T3B => "3",
        }
    }
}

/// Adjustment types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuType {
    Qfq,
    Hfq,
    None,
}

/// Trend types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrendType {
    Mean,
    Max,
    Min,
}

impl FromStr for TrendType {
    type Err = PyErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mean" => Ok(TrendType::Mean),
            "max" => Ok(TrendType::Max),
            "min" => Ok(TrendType::Min),
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid trend type: {}", s),
            )),
        }
    }
}

/// Trend line side types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrendLineSide {
    Inside,
    Outside,
}

/// Left segment method types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeftSegMethod {
    All,
    Peak,
}

/// FX check method types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxCheckMethod {
    Strict,
    Loss,
    Half,
    Totally,
}

/// Segment types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegType {
    Bi,
    Seg,
}

/// MACD algorithm types
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacdAlgo {
    Area,
    Peak,
    FullArea,
    Diff,
    Slope,
    Amp,
    Volume,
    Amount,
    VolumeAvg,
    AmountAvg,
    TurnrateAvg,
    Rsi,
}

/// Data field constants
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataField {
    Time,
    Open,
    High,
    Low,
    Close,
    Volume,     // 成交量
    Turnover,   // 成交额
    TurnRate,   // 换手率
}

// Python bindings
#[pymethods]
impl DataField {
    #[classattr]
    const FIELD_TIME: &'static str = "time_key";
    #[classattr]
    const FIELD_OPEN: &'static str = "open";
    #[classattr]
    const FIELD_HIGH: &'static str = "high";
    #[classattr]
    const FIELD_LOW: &'static str = "low";
    #[classattr]
    const FIELD_CLOSE: &'static str = "close";
    #[classattr]
    const FIELD_VOLUME: &'static str = "volume";
    #[classattr]
    const FIELD_TURNOVER: &'static str = "turnover";
    #[classattr]
    const FIELD_TURNRATE: &'static str = "turnover_rate";
}

// Trade info list constant
pub const TRADE_INFO_LIST: &[DataField] = &[
    DataField::Volume,
    DataField::Turnover,
    DataField::TurnRate,
];

// Python module initialization
#[pymodule]
fn chan_enums(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<DataSource>()?;
    m.add_class::<KLineType>()?;
    m.add_class::<KLineDir>()?;
    m.add_class::<FxType>()?;
    m.add_class::<BiDir>()?;
    m.add_class::<BiType>()?;
    m.add_class::<BspType>()?;
    m.add_class::<AuType>()?;
    m.add_class::<TrendType>()?;
    m.add_class::<TrendLineSide>()?;
    m.add_class::<LeftSegMethod>()?;
    m.add_class::<FxCheckMethod>()?;
    m.add_class::<SegType>()?;
    m.add_class::<MacdAlgo>()?;
    m.add_class::<DataField>()?;
    Ok(())
} 