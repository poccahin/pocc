use std::f64::consts::E;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tonic::transport::Channel;

// 引入 gRPC 客户端
pub mod pocc {
    tonic::include_proto!("pocc");
}
use pocc::tensor_validator_client::TensorValidatorClient;
use pocc::TensorRequest;

// --- 数据结构 ---

#[derive(Debug, Clone)]
pub struct AgentNode {
    pub node_id: String,
    pub entropy_reduction_joules: f64,
    pub life_plus_staked: f64,
    pub topological_entropy: f64,
}

pub struct GravityRouter {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
}

impl GravityRouter {
    pub fn calculate_gravity(&self, target: &AgentNode, semantic_distance: f64) -> f64 {
        let economic_mass = self.beta * target.life_plus_staked.sqrt();
        let physical_mass = self.alpha * target.entropy_reduction_joules;
        let total_mass = physical_mass + economic_mass;
        let distance_sq = semantic_distance.powi(2).max(1e-6);
        let monopoly_decay = E.powf(self.gamma * target.topological_entropy);
        total_mass / (distance_sq * monopoly_decay)
    }
}

// 接收外部网络的 JSON 格式张量请求
#[derive(Deserialize, Debug)]
struct IncomingIntent {
    agent_id: String,
    intent_tensor: Vec<f32>,
    target_semantic_distance: f64, // 简化：模拟该任务与现有节点群的平均语义距离
}

// 返回给发起者的 JSON 路由结果
#[derive(Serialize, Debug)]
struct RoutingResponse {
    status: String,
    assigned_node: Option<String>,
    message: String,
}

// --- 核心守护进程 ---

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🪐 [Life++ OS] Booting AHIN L1 Daemon...");

    // 1. 初始化全局节点状态与路由器 (使用 Arc 跨协程共享)
    let router = Arc::new(GravityRouter {
        alpha: 1.5,
        beta: 1.0,
        gamma: 2.0,
    });
    let nodes = Arc::new(vec![
        AgentNode {
            node_id: "CAI-01-财团寡头".into(),
            entropy_reduction_joules: 100.0,
            life_plus_staked: 1_000_000.0,
            topological_entropy: 3.5,
        },
        AgentNode {
            node_id: "CAI-02-实干型机器人".into(),
            entropy_reduction_joules: 8500.0,
            life_plus_staked: 500.0,
            topological_entropy: 0.2,
        },
    ]);

    // 2. 建立与 Python (L2.5) 的单一多路复用 HTTP/2 gRPC 通道
    println!("🔗 Connecting to L2.5 Tensor Validator (Python)...");
    let channel = Channel::from_static("http://127.0.0.1:50051")
        .connect()
        .await?;
    let grpc_client = TensorValidatorClient::new(channel);

    // 3. 绑定 TCP 监听器 (对外暴露的 P2P 端口)
    let listener = TcpListener::bind("0.0.0.0:8000").await?;
    println!("🚀 [AHIN Daemon] Online and listening on 0.0.0.0:8000\n");

    // 4. 无限并发事件循环 (The Infinite Event Loop)
    loop {
        // 挂起等待新的网络连接
        let (socket, addr) = listener.accept().await?;
        println!("📡 [Network] Connection established from: {}", addr);

        // 为每个连接低成本克隆上下文 (只增加引用计数)
        let router_clone = Arc::clone(&router);
        let nodes_clone = Arc::clone(&nodes);
        let client_clone = grpc_client.clone();

        // 🌟 并发魔法：将该请求丢给 Tokio 协程池去异步处理，绝对不阻塞主线程！
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, router_clone, nodes_clone, client_clone).await
            {
                eprintln!("⚠️ [Network Error] Peer dropped: {}", e);
            }
        });
    }
}

/// 独立的异步协程：处理单个张量请求的完整生命周期
async fn handle_connection(
    socket: TcpStream,
    router: Arc<GravityRouter>,
    nodes: Arc<Vec<AgentNode>>,
    mut grpc_client: TensorValidatorClient<Channel>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = socket.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    // 读取一行 JSON (TCP 流式读取)
    buf_reader.read_line(&mut line).await?;
    if line.trim().is_empty() {
        return Ok(());
    }

    // 1. 反序列化
    let intent: IncomingIntent = serde_json::from_str(&line)?;
    println!(
        "🔍 [Router] Received Tensor from [{}] (Dim: {})",
        intent.agent_id,
        intent.intent_tensor.len()
    );

    // 2. 防火墙拦截：调用 gRPC 请求 Python 进行蒙特卡洛噪声测试
    let request = tonic::Request::new(TensorRequest {
        agent_id: intent.agent_id.clone(),
        intent_tensor: intent.intent_tensor,
    });

    let response = grpc_client.verify_robustness(request).await?.into_inner();

    let reply = if !response.is_robust {
        println!(
            "💀 [FATAL] Tensor Poisoning intercepted for {}. Slashing protocol armed.",
            intent.agent_id
        );
        RoutingResponse {
            status: "REJECTED_ADVERSARIAL_SPIKE".to_string(),
            assigned_node: None,
            message: response.diagnosis,
        }
    } else {
        println!("✅ [PoCC] Tensor robust. Executing CR+ Gravity calculation...");
        // 3. 物理路由计算
        let mut best_node = None;
        let mut max_gravity = -1.0;

        for node in nodes.iter() {
            let gravity = router.calculate_gravity(node, intent.target_semantic_distance);
            if gravity > max_gravity {
                max_gravity = gravity;
                best_node = Some(node.node_id.clone());
            }
        }

        RoutingResponse {
            status: "ROUTED_SUCCESSFULLY".to_string(),
            assigned_node: best_node,
            message: format!(
                "Semantic drift: {:.4}. Routed via CR+ gravity.",
                response.drift_variance
            ),
        }
    };

    // 4. 将路由结果序列化为 JSON 并发回给客户端
    let response_json = serde_json::to_string(&reply)? + "\n";
    writer.write_all(response_json.as_bytes()).await?;

    Ok(())
}
