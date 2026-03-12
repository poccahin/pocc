use std::f64::consts::E;

#[derive(Debug, Clone)]
pub struct LocalGridMetrics {
    pub geohash_zone: String,
    pub compute_idle_ratio: f64, // 0.0 到 1.0 (例如 0.8 表示 80% 的 Mac mini 处于闲置)
    pub power_grid_load: f64,    // 0.0 到 1.0 (例如 0.95 表示电网即将熔断)
}

impl LocalGridMetrics {
    /// 计算动态燃烧税 (Dynamic Burn Tax) - 万分比 (BIPS)
    ///
    /// 模型:
    /// Tax_burn = BaseTax + α * e^(β * GridLoad) - γ * IdleRatio
    pub fn calculate_dynamic_burn_bips(&self) -> u64 {
        let base_tax_bips = 100.0; // 基准线 1%

        // 宏观调控系数
        let alpha = 50.0;
        let beta = 4.0;
        let gamma = 80.0;

        // 1. 电网过载的指数级惩罚 (Self-Affirmation 防御机制)
        let thermal_penalty = alpha * E.powf(beta * self.power_grid_load);

        // 2. 算力过剩的线性补贴 (刺激扩张)
        let idle_subsidy = gamma * self.compute_idle_ratio;

        // 3. 融合计算
        let dynamic_tax = base_tax_bips + thermal_penalty - idle_subsidy;

        // 边界钳制：最低不低于 0.01% (防止防女巫机制失效)，最高不超过 50% (物理熔断)
        let clamped_tax = dynamic_tax.clamp(1.0, 5000.0);

        clamped_tax as u64
    }
}

#[cfg(test)]
mod tests {
    use super::LocalGridMetrics;

    #[test]
    fn tax_increases_when_grid_load_rises() {
        let low_load = LocalGridMetrics {
            geohash_zone: "wx4g0".to_string(),
            compute_idle_ratio: 0.1,
            power_grid_load: 0.2,
        };
        let high_load = LocalGridMetrics {
            geohash_zone: "wx4g0".to_string(),
            compute_idle_ratio: 0.1,
            power_grid_load: 0.9,
        };

        assert!(
            high_load.calculate_dynamic_burn_bips() > low_load.calculate_dynamic_burn_bips()
        );
    }

    #[test]
    fn tax_decreases_when_idle_ratio_rises() {
        let low_idle = LocalGridMetrics {
            geohash_zone: "wx4g0".to_string(),
            compute_idle_ratio: 0.1,
            power_grid_load: 0.5,
        };
        let high_idle = LocalGridMetrics {
            geohash_zone: "wx4g0".to_string(),
            compute_idle_ratio: 0.9,
            power_grid_load: 0.5,
        };

        assert!(
            high_idle.calculate_dynamic_burn_bips() < low_idle.calculate_dynamic_burn_bips()
        );
    }

    #[test]
    fn tax_is_clamped_to_protocol_bounds() {
        let cooling = LocalGridMetrics {
            geohash_zone: "wx4g0".to_string(),
            compute_idle_ratio: 1.0,
            power_grid_load: 0.0,
        };
        let meltdown = LocalGridMetrics {
            geohash_zone: "wx4g0".to_string(),
            compute_idle_ratio: 0.0,
            power_grid_load: 1.0,
        };

        assert!(cooling.calculate_dynamic_burn_bips() >= 1);
        assert!(meltdown.calculate_dynamic_burn_bips() <= 5000);
    }
}
