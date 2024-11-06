use pyo3::prelude::*;
use crate::common::enums::FxCheckMethod;
use crate::common::error::{ChanException, ErrCode};

/// Configuration for Bi (笔) analysis
#[pyclass]
#[derive(Debug, Clone)]
pub struct BiConfig {
    pub bi_algo: String,             // 笔算法
    pub is_strict: bool,             // 是否严格模式
    pub bi_fx_check: FxCheckMethod,  // 分型检查方法
    pub gap_as_kl: bool,            // 是否将缺口视为K线
    pub bi_end_is_peak: bool,       // 笔的结束是否必须是峰
    pub bi_allow_sub_peak: bool,    // 是否允许次级别分型
}

#[pymethods]
impl BiConfig {
    /// Create a new BiConfig instance
    #[new]
    #[pyo3(signature = (
        bi_algo="normal",
        is_strict=true,
        bi_fx_check="half",
        gap_as_kl=true,
        bi_end_is_peak=true,
        bi_allow_sub_peak=true
    ))]
    pub fn new(
        bi_algo: &str,
        is_strict: bool,
        bi_fx_check: &str,
        gap_as_kl: bool,
        bi_end_is_peak: bool,
        bi_allow_sub_peak: bool,
    ) -> PyResult<Self> {
        let bi_fx_check = match bi_fx_check {
            "strict" => FxCheckMethod::Strict,
            "loss" => FxCheckMethod::Loss,
            "half" => FxCheckMethod::Half,
            "totally" => FxCheckMethod::Totally,
            _ => return Err(ChanException::new(
                format!("unknown bi_fx_check={}", bi_fx_check),
                ErrCode::ParaError
            ).into())
        };

        Ok(Self {
            bi_algo: bi_algo.to_string(),
            is_strict,
            bi_fx_check,
            gap_as_kl,
            bi_end_is_peak,
            bi_allow_sub_peak,
        })
    }

    /// Get bi algorithm
    #[getter]
    pub fn get_bi_algo(&self) -> String {
        self.bi_algo.clone()
    }

    /// Get strict mode status
    #[getter]
    pub fn get_is_strict(&self) -> bool {
        self.is_strict
    }

    /// Get fractal check method
    #[getter]
    pub fn get_bi_fx_check(&self) -> FxCheckMethod {
        self.bi_fx_check
    }

    /// Get gap as kline status
    #[getter]
    pub fn get_gap_as_kl(&self) -> bool {
        self.gap_as_kl
    }

    /// Get bi end is peak status
    #[getter]
    pub fn get_bi_end_is_peak(&self) -> bool {
        self.bi_end_is_peak
    }

    /// Get bi allow sub peak status
    #[getter]
    pub fn get_bi_allow_sub_peak(&self) -> bool {
        self.bi_allow_sub_peak
    }

    /// String representation
    fn __str__(&self) -> String {
        format!(
            "BiConfig(bi_algo={}, is_strict={}, bi_fx_check={:?}, gap_as_kl={}, bi_end_is_peak={}, bi_allow_sub_peak={})",
            self.bi_algo,
            self.is_strict,
            self.bi_fx_check,
            self.gap_as_kl,
            self.bi_end_is_peak,
            self.bi_allow_sub_peak
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BiConfig::new(
            "normal",
            true,
            "half",
            true,
            true,
            true
        ).unwrap();

        assert_eq!(config.bi_algo, "normal");
        assert!(config.is_strict);
        assert_eq!(config.bi_fx_check, FxCheckMethod::Half);
        assert!(config.gap_as_kl);
        assert!(config.bi_end_is_peak);
        assert!(config.bi_allow_sub_peak);
    }

    #[test]
    fn test_custom_config() {
        let config = BiConfig::new(
            "custom",
            false,
            "strict",
            false,
            false,
            false
        ).unwrap();

        assert_eq!(config.bi_algo, "custom");
        assert!(!config.is_strict);
        assert_eq!(config.bi_fx_check, FxCheckMethod::Strict);
        assert!(!config.gap_as_kl);
        assert!(!config.bi_end_is_peak);
        assert!(!config.bi_allow_sub_peak);
    }

    #[test]
    fn test_invalid_fx_check() {
        let result = BiConfig::new(
            "normal",
            true,
            "invalid",
            true,
            true,
            true
        );

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("unknown bi_fx_check"));
        }
    }

    #[test]
    fn test_string_representation() {
        let config = BiConfig::new(
            "normal",
            true,
            "half",
            true,
            true,
            true
        ).unwrap();

        let str_rep = config.__str__();
        assert!(str_rep.contains("bi_algo=normal"));
        assert!(str_rep.contains("is_strict=true"));
        assert!(str_rep.contains("bi_fx_check=Half"));
    }
} 