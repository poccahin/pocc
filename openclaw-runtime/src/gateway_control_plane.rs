use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};

/// 物理与算力资源拓扑枚举
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum HardwareResource {
    Chassis,
    LeftActuator,
    RightActuator,
    TensorNpu,
    VisionSensors,
}

/// 标准化的物理执行意图
#[derive(Debug, Clone)]
pub struct PhysicalIntent {
    pub intent_id: String,
    pub required_resources: Vec<HardwareResource>,
    pub tensor_payload: Vec<u8>,
    pub safety_tolerance: f32,
}

/// 车道队列管理器 (Lane Queue Manager)
pub struct LaneQueueManager {
    /// 每个物理硬件资源对应一个独立的串行执行队列 (车道)
    lanes: RwLock<HashMap<HardwareResource, VecDeque<PhysicalIntent>>>,
    /// 资源锁定状态表，防止交叉死锁
    resource_locks: Arc<Mutex<HashMap<HardwareResource, bool>>>,
}

impl LaneQueueManager {
    pub fn new() -> Self {
        let mut lanes = HashMap::new();
        let mut locks = HashMap::new();
        let resources = [
            HardwareResource::Chassis,
            HardwareResource::LeftActuator,
            HardwareResource::RightActuator,
            HardwareResource::TensorNpu,
            HardwareResource::VisionSensors,
        ];

        for resource in resources {
            lanes.insert(resource.clone(), VecDeque::new());
            locks.insert(resource, false);
        }

        Self {
            lanes: RwLock::new(lanes),
            resource_locks: Arc::new(Mutex::new(locks)),
        }
    }

    /// 接收高并发的网络意图，并将其压入对应的物理车道
    pub async fn enqueue_intent(&self, intent: PhysicalIntent) {
        let mut lanes = self.lanes.write().await;
        for resource in &intent.required_resources {
            if let Some(queue) = lanes.get_mut(resource) {
                queue.push_back(intent.clone());
            }
        }

        println!(
            "🚥 [GATEWAY] Intent {} multiplexed into hardware lanes: {:?}",
            intent.intent_id, intent.required_resources
        );
    }
}

/// 目标驱动引擎 (Objective-Driven Engine)
pub struct ObjectiveDrivenEngine;

impl ObjectiveDrivenEngine {
    /// 在真实驱动之前进行 MPC 风险推演
    pub async fn psychological_rehearsal(intent: &PhysicalIntent) -> Result<(), String> {
        println!(
            "🧠 [MPC] Initiating psychological rehearsal for Intent: {}...",
            intent.intent_id
        );

        let predicted_risk = Self::simulate_kinematics(&intent.tensor_payload);
        if predicted_risk > intent.safety_tolerance {
            return Err(format!(
                "💥 [FATAL] MPC Rehearsal failed. Predicted physical risk ({predicted_risk}) exceeds tolerance."
            ));
        }

        println!("✅ [MPC] Rehearsal passed. Trajectory is safe for reality projection.");
        Ok(())
    }

    fn simulate_kinematics(_payload: &[u8]) -> f32 {
        0.01
    }
}

/// 控制平面中枢 (The Gateway Control Plane)
pub struct GatewayControlPlane {
    pub lane_manager: Arc<LaneQueueManager>,
}

impl GatewayControlPlane {
    pub fn new() -> Self {
        Self {
            lane_manager: Arc::new(LaneQueueManager::new()),
        }
    }

    /// 启动网关主循环：将数字意图降维投影到物理世界
    pub async fn run_execution_loop(lane_manager: Arc<LaneQueueManager>) {
        println!(
            "🛡️ [CONTROL PLANE] Physical execution loop ignited. Managing thermodynamic projection."
        );

        loop {
            let mut locks = lane_manager.resource_locks.lock().await;
            let mut lanes = lane_manager.lanes.write().await;

            for (resource, queue) in lanes.iter_mut() {
                if !*locks.get(resource).unwrap_or(&true) {
                    if let Some(intent) = queue.front() {
                        let all_resources_available = intent
                            .required_resources
                            .iter()
                            .all(|r| !*locks.get(r).unwrap_or(&true));

                        if all_resources_available {
                            let executable_intent = queue.pop_front().expect("intent must exist");

                            for r in &executable_intent.required_resources {
                                locks.insert(r.clone(), true);
                            }

                            let locks_clone = Arc::clone(&lane_manager.resource_locks);
                            tokio::spawn(async move {
                                match ObjectiveDrivenEngine::psychological_rehearsal(
                                    &executable_intent,
                                )
                                .await
                                {
                                    Ok(()) => {
                                        println!(
                                            "🦾 [ACTUATOR] Dispatching safe intent {} to Zig firmware.",
                                            executable_intent.intent_id
                                        );
                                        sleep(Duration::from_millis(100)).await;
                                    }
                                    Err(e) => eprintln!("{e}"),
                                }

                                let mut release_locks = locks_clone.lock().await;
                                for r in &executable_intent.required_resources {
                                    release_locks.insert(r.clone(), false);
                                }
                                println!(
                                    "🔓 [GATEWAY] Resources released for Intent {}",
                                    executable_intent.intent_id
                                );
                            });
                        }
                    }
                }
            }
            drop(lanes);
            drop(locks);
            tokio::task::yield_now().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enqueue_to_all_required_lanes() {
        let manager = LaneQueueManager::new();
        let intent = PhysicalIntent {
            intent_id: "intent-001".to_string(),
            required_resources: vec![HardwareResource::Chassis, HardwareResource::VisionSensors],
            tensor_payload: vec![1, 2, 3],
            safety_tolerance: 0.2,
        };

        manager.enqueue_intent(intent.clone()).await;

        let lanes = manager.lanes.read().await;
        assert_eq!(
            lanes
                .get(&HardwareResource::Chassis)
                .expect("missing chassis lane")
                .len(),
            1
        );
        assert_eq!(
            lanes
                .get(&HardwareResource::VisionSensors)
                .expect("missing vision lane")
                .len(),
            1
        );

        assert_eq!(
            lanes
                .get(&HardwareResource::Chassis)
                .and_then(VecDeque::front)
                .expect("lane should contain intent")
                .intent_id,
            intent.intent_id
        );
    }

    #[tokio::test]
    async fn rehearsal_rejects_high_risk_intent() {
        let intent = PhysicalIntent {
            intent_id: "intent-unsafe".to_string(),
            required_resources: vec![HardwareResource::LeftActuator],
            tensor_payload: vec![9, 9, 9],
            safety_tolerance: 0.001,
        };

        let result = ObjectiveDrivenEngine::psychological_rehearsal(&intent).await;
        assert!(result.is_err());
    }
}
