impl TradeInfo {
    pub fn set_macd(&mut self, macd: MacdItem) {
        self.macd = Some(macd);
    }

    pub fn set_boll(&mut self, boll: BollMetric) {
        self.boll = Some(boll);
    }

    pub fn set_kdj(&mut self, k: f64, d: f64, j: f64) {
        self.kdj = Some((k, d, j));
    }

    pub fn set_rsi(&mut self, rsi: f64) {
        self.rsi = Some(rsi);
    }
} 