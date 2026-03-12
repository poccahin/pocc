use std::cmp::Ordering;
use std::f64::consts::PI;

/// 智能体的物理空间信标
#[derive(Debug, Clone)]
pub struct SpatialBeacon {
    pub lat: f64,
    pub lon: f64,
    pub geohash: String, // 例如 "ws12" 代表深圳某片区
}

#[derive(Debug, Clone)]
pub struct CyberneticNode {
    pub did: String,
    pub node_type: NodeType,
    pub beacon: SpatialBeacon,
    pub scog_score: u64,
    pub staked_life_plus: u64,
    pub current_ping_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    MacMiniCortex,    // 大脑皮层：提供高维多模态算力
    HumanoidActuator, // 执行器：人型机器人终端
}

impl CyberneticNode {
    /// 计算地球表面两点之间的 Haversine 物理距离 (单位：千米)
    pub fn haversine_distance(&self, other: &SpatialBeacon) -> f64 {
        let earth_radius_km = 6371.0;
        let d_lat = (other.lat - self.beacon.lat) * PI / 180.0;
        let d_lon = (other.lon - self.beacon.lon) * PI / 180.0;

        let lat1 = self.beacon.lat * PI / 180.0;
        let lat2 = other.lat * PI / 180.0;

        let a = (d_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        earth_radius_km * c
    }

    /// 核心算法：计算基于“本地重力场”的空间引力
    /// 物理法则：距离越远，引力呈指数级衰减
    pub fn calculate_spatial_gravity(&self, target_robot: &SpatialBeacon) -> f64 {
        // 1. 基础认知资本与质押权重
        let base_mass = (self.scog_score as f64) * 1.5
            + if self.staked_life_plus > 0 {
                (self.staked_life_plus as f64).log10() * 2.0
            } else {
                0.0
            };

        // 2. 物理距离计算 (Km)
        let distance_km = self.haversine_distance(target_robot);

        // 🚨 3. 空间折叠核心：极端的地理惩罚 (Geo-Penalty)
        // 500米以内：几乎无惩罚，属于同一个“蜂群神经节”
        // 超过10公里：引力断崖式暴跌，强制切断跨城高延迟调度
        let spatial_penalty = if distance_km <= 0.5 {
            1.0 // 完美局域网
        } else {
            // 指数衰减公式：e^(-k * distance)
            (-0.8 * distance_km).exp()
        };

        // 4. 网络延迟硬性熔断
        let latency_penalty = if self.current_ping_ms > 20 {
            0.01 // 超过 20ms 的节点直接被踢出反射弧候选列表
        } else {
            1.0 / (self.current_ping_ms as f64).max(1.0)
        };

        base_mass * spatial_penalty * latency_penalty
    }
}

/// 蜂群神经节 (Swarm Ganglia) 路由制导
/// 当机器人请求高维认知时，在 Geohash 匹配的本地网格中寻找最强 Mac mini
pub fn route_to_local_cortex(
    robot_beacon: &SpatialBeacon,
    available_cortex_nodes: &[CyberneticNode],
) -> Option<CyberneticNode> {
    let prefix = &robot_beacon.geohash[..robot_beacon.geohash.len().min(4)];

    available_cortex_nodes
        .iter()
        .filter(|node| node.node_type == NodeType::MacMiniCortex)
        // 粗筛：利用 Geohash 前缀匹配，瞬间过滤掉非本城市的节点，将 O(N) 降维
        .filter(|node| node.beacon.geohash.starts_with(prefix))
        // 精筛：在本地重力场内，寻找空间引力最大的节点
        .max_by(|a, b| {
            let gravity_a = a.calculate_spatial_gravity(robot_beacon);
            let gravity_b = b.calculate_spatial_gravity(robot_beacon);
            gravity_a.partial_cmp(&gravity_b).unwrap_or(Ordering::Equal)
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::{route_to_local_cortex, CyberneticNode, NodeType, SpatialBeacon};

    fn mk_cortex(
        did: &str,
        geohash: &str,
        lat: f64,
        lon: f64,
        scog_score: u64,
        staked_life_plus: u64,
        current_ping_ms: u64,
    ) -> CyberneticNode {
        CyberneticNode {
            did: did.to_string(),
            node_type: NodeType::MacMiniCortex,
            beacon: SpatialBeacon {
                lat,
                lon,
                geohash: geohash.to_string(),
            },
            scog_score,
            staked_life_plus,
            current_ping_ms,
        }
    }

    #[test]
    fn route_prefers_local_low_latency_cortex() {
        let robot = SpatialBeacon {
            lat: 22.5431,
            lon: 114.0579,
            geohash: "ws1056".to_string(),
        };

        let nodes = vec![
            mk_cortex("did:ahin:shenzhen-a", "ws1051", 22.5432, 114.0580, 160, 20_000, 8),
            mk_cortex("did:ahin:shenzhen-b", "ws1052", 22.5433, 114.0577, 130, 22_000, 6),
            mk_cortex("did:ahin:nyc", "dr5reg", 40.7128, -74.0060, 500, 100_000, 18),
        ];

        let selected = route_to_local_cortex(&robot, &nodes).expect("should route to a local cortex");
        assert_eq!(selected.did, "did:ahin:shenzhen-b");
    }

    #[test]
    fn route_handles_short_geohash_prefix() {
        let robot = SpatialBeacon {
            lat: 22.5431,
            lon: 114.0579,
            geohash: "ws".to_string(),
        };

        let nodes = vec![mk_cortex(
            "did:ahin:shenzhen-a",
            "ws1051",
            22.5432,
            114.0580,
            160,
            20_000,
            8,
        )];

        let selected = route_to_local_cortex(&robot, &nodes);
        assert!(selected.is_some());
    }

    #[test]
    fn high_latency_nodes_are_strongly_penalized() {
        let robot = SpatialBeacon {
            lat: 22.5431,
            lon: 114.0579,
            geohash: "ws1056".to_string(),
        };

        let low_latency = mk_cortex("did:ahin:fast", "ws1056", 22.5431, 114.0579, 120, 2_000, 10);
        let high_latency = mk_cortex("did:ahin:slow", "ws1056", 22.5431, 114.0579, 120, 2_000, 45);

        assert!(
            low_latency.calculate_spatial_gravity(&robot)
                > high_latency.calculate_spatial_gravity(&robot)
        );
    }
}
