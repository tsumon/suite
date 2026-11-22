#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceSnapshot {
    pub if_index: u32,
    pub alias: String,
    pub description: String,
    pub is_up: bool,
    pub if_type: u32,
    pub media_connected: bool,
    pub in_octets: u64,
    pub out_octets: u64,
}

impl InterfaceSnapshot {
    pub fn is_loopback(&self) -> bool {
        self.if_type == 24 // IF_TYPE_SOFTWARE_LOOPBACK
    }

    pub fn is_tunnel(&self) -> bool {
        self.if_type == 131 // IF_TYPE_TUNNEL
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RateSample {
    pub if_index: u32,
    pub alias: String,
    pub receive_bytes_per_second: f64,
    pub send_bytes_per_second: f64,
    pub adapter_missing: bool,
}

pub trait InterfaceTable: Send + Sync {
    fn get_table(&self) -> Result<Vec<InterfaceSnapshot>, String>;
    fn get_entry(&self, if_index: u32) -> Option<InterfaceSnapshot>;
}

pub struct RateCalculator;

impl RateCalculator {
    pub fn try_compute(
        previous: &InterfaceSnapshot,
        current: &InterfaceSnapshot,
        elapsed_seconds: f64,
    ) -> Option<(f64, f64)> {
        if elapsed_seconds <= 0.0 || !elapsed_seconds.is_finite() {
            return None;
        }
        if current.if_index != previous.if_index {
            return None;
        }
        if current.in_octets < previous.in_octets || current.out_octets < previous.out_octets {
            return None;
        }
        let rx = (current.in_octets - previous.in_octets) as f64 / elapsed_seconds;
        let tx = (current.out_octets - previous.out_octets) as f64 / elapsed_seconds;
        Some((rx, tx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(idx: u32, rx: u64, tx: u64) -> InterfaceSnapshot {
        InterfaceSnapshot {
            if_index: idx,
            alias: "WLAN".into(),
            description: "Wi-Fi".into(),
            is_up: true,
            if_type: 71,
            media_connected: true,
            in_octets: rx,
            out_octets: tx,
        }
    }

    #[test]
    fn rate_calculator_guards_counter_wrap_and_zero_elapsed() {
        let a = snap(1, 1000, 500);
        let b = snap(1, 3000, 1500);
        assert_eq!(RateCalculator::try_compute(&a, &b, 2.0), Some((1000.0, 500.0)));
        assert_eq!(RateCalculator::try_compute(&a, &b, 0.0), None);
        assert_eq!(RateCalculator::try_compute(&b, &a, 1.0), None);
    }
}
