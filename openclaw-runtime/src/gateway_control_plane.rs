use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::task;

/// 使用位图 (Bitmask) 重新定义物理资源，便于进行极速原子操作
pub const RES_CHASSIS: u8 = 0b0000_0001;
pub const RES_LEFT_ARM: u8 = 0b0000_0010;
pub const RES_RIGHT_ARM: u8 = 0b0000_0100;
pub const RES_TENSOR_NPU: u8 = 0b0000_1000;
pub const RES_VISION: u8 = 0b0001_0000;

#[derive(Debug, Clone)]
pub struct PhysicalIntent {
    pub intent_id: String,
    pub resource_mask: u8,
    pub tensor_payload: Vec<u8>,
}

/// 绝对无锁的车道与资源管理器
pub struct LockFreeLaneManager {
    /// 物理资源全局原子锁 (位图)
    global_resource_state: AtomicU8,
    /// 高速 MPMC 环形缓冲区
    intent_tx: Sender<PhysicalIntent>,
    intent_rx: Receiver<PhysicalIntent>,
}

impl LockFreeLaneManager {
    pub fn new() -> Self {
        let (tx, rx) = bounded(65_536);
        Self {
            global_resource_state: AtomicU8::new(0),
            intent_tx: tx,
            intent_rx: rx,
        }
    }

    /// 纳秒级意图压入，不阻塞 P2P 网络事件循环
    pub fn enqueue_intent(&self, intent: PhysicalIntent) {
        if self.intent_tx.try_send(intent).is_err() {
            #[cfg(debug_assertions)]
            eprintln!("🔥 [GATEWAY WARNING] Lane queue overflow. Intent dropped.");
        }
    }

    /// 尝试原子化地锁定所有需要的资源
    pub fn try_acquire_resources(&self, mask: u8) -> bool {
        let mut current = self.global_resource_state.load(Ordering::Acquire);
        loop {
            if current & mask != 0 {
                return false;
            }

            let new_state = current | mask;
            match self.global_resource_state.compare_exchange_weak(
                current,
                new_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// 释放资源
    pub fn release_resources(&self, mask: u8) {
        self.global_resource_state
            .fetch_and(!mask, Ordering::Release);
    }

    #[cfg(test)]
    fn drain_one(&self) -> Option<PhysicalIntent> {
        self.intent_rx.try_recv().ok()
    }
}

/// 控制平面执行主循环
pub async fn run_lockfree_execution_loop(manager: Arc<LockFreeLaneManager>) {
    task::spawn_blocking(move || {
        println!("🚀 [CONTROL PLANE] Lock-free thermodynamic projection loop activated.");

        loop {
            if let Ok(intent) = manager.intent_rx.recv() {
                if manager.try_acquire_resources(intent.resource_mask) {
                    let mgr_clone = manager.clone();
                    rayon::spawn(move || {
                        println!(
                            "🦾 [ACTUATOR] Executing intent {} on mask {:b}",
                            intent.intent_id, intent.resource_mask
                        );
                        let _ = intent.tensor_payload;
                        mgr_clone.release_resources(intent.resource_mask);
                    });
                } else {
                    let _ = manager.intent_tx.try_send(intent);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_pop_intent_from_lockfree_queue() {
        let manager = LockFreeLaneManager::new();
        let intent = PhysicalIntent {
            intent_id: "intent-001".to_string(),
            resource_mask: RES_CHASSIS | RES_VISION,
            tensor_payload: vec![1, 2, 3],
        };

        manager.enqueue_intent(intent.clone());

        let pulled = manager
            .drain_one()
            .expect("intent should be available in queue");
        assert_eq!(pulled.intent_id, intent.intent_id);
        assert_eq!(pulled.resource_mask, intent.resource_mask);
    }

    #[test]
    fn resource_acquire_conflict_and_release() {
        let manager = LockFreeLaneManager::new();
        let mask = RES_LEFT_ARM | RES_TENSOR_NPU;

        assert!(manager.try_acquire_resources(mask));
        assert!(!manager.try_acquire_resources(RES_LEFT_ARM));

        manager.release_resources(mask);

        assert!(manager.try_acquire_resources(RES_LEFT_ARM));
    }
}
