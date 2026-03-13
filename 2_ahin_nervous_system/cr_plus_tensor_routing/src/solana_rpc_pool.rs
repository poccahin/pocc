use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// 高可用 RPC 轮询阵列：绝不让任何一笔轧账失败
pub struct RpcFailoverPool {
    endpoints: Vec<String>,
    current_index: AtomicUsize,
}

impl RpcFailoverPool {
    pub fn new(endpoints: Vec<&str>) -> Arc<Self> {
        assert!(
            !endpoints.is_empty(),
            "at least one RPC endpoint is required"
        );

        Arc::new(Self {
            endpoints: endpoints.into_iter().map(str::to_string).collect(),
            current_index: AtomicUsize::new(0),
        })
    }

    /// 获取当前激活的 RPC 客户端
    fn get_current_client(&self) -> RpcClient {
        let index = self.current_index.load(Ordering::Acquire) % self.endpoints.len();
        let url = &self.endpoints[index];
        RpcClient::new_with_commitment(url.to_string(), CommitmentConfig::confirmed())
    }

    /// 轮询切换到下一个备用节点
    fn rotate_endpoint(&self) {
        let old_index = self.current_index.fetch_add(1, Ordering::Release);
        let new_index = (old_index + 1) % self.endpoints.len();
        println!(
            "⚠️ [RPC Monitor] HTTP 429/503 Detected. Rotating endpoint to: {}",
            self.endpoints[new_index]
        );
    }

    /// 核心包装器：带自动容错与指数退避的闭包执行引擎
    pub async fn execute_with_failover<F, Fut, T, E>(&self, mut operation: F) -> Result<T, String>
    where
        F: FnMut(RpcClient) -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let max_retries = 5;
        let mut base_delay_ms = 100;

        for attempt in 0..max_retries {
            let client = self.get_current_client();

            match operation(client).await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    let err_msg = format!("{:?}", err);
                    // 精准捕获限流与节点宕机错误
                    if err_msg.contains("429")
                        || err_msg.contains("503")
                        || err_msg.to_ascii_lowercase().contains("timeout")
                    {
                        println!(
                            "🚨 [RPC Pool] Node choked on attempt {}/{}. Error: {:?}",
                            attempt + 1,
                            max_retries,
                            err_msg
                        );

                        self.rotate_endpoint();

                        // 指数退避策略：防止新节点也被瞬间击穿
                        sleep(Duration::from_millis(base_delay_ms)).await;
                        base_delay_ms *= 2;
                        continue;
                    }

                    // 合约逻辑报错：不重试
                    return Err(format!("Fatal contract error: {:?}", err_msg));
                }
            }
        }

        Err(format!(
            "❌ [CRITICAL] All RPC endpoints choked after {} retries. Netting transaction aborted.",
            max_retries
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retries_transient_errors_then_succeeds() {
        let pool = RpcFailoverPool::new(vec!["https://rpc-1.test", "https://rpc-2.test"]);
        let mut attempts = 0;

        let result = pool
            .execute_with_failover(|_client| {
                attempts += 1;
                async move {
                    if attempts < 3 {
                        Err("HTTP 429")
                    } else {
                        Ok("tx-ok")
                    }
                }
            })
            .await;

        assert_eq!(result.expect("should eventually succeed"), "tx-ok");
    }

    #[tokio::test]
    async fn fails_fast_for_non_retryable_errors() {
        let pool = RpcFailoverPool::new(vec!["https://rpc-1.test"]);

        let err = pool
            .execute_with_failover(|_client| async { Err::<(), _>("insufficient funds") })
            .await
            .expect_err("should fail without retry");

        assert!(err.contains("Fatal contract error"));
    }
}
