use futures::StreamExt;
use libp2p::swarm::{StreamProtocol, SwarmEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::AhinBehaviour;

/// 机器人的本地控制台或硬件 API 端口 (永远不对外网暴露)
const LOCAL_HARDWARE_API: &str = "127.0.0.1:8080";
const AHIN_TUNNEL_PROTOCOL: StreamProtocol = StreamProtocol::new("/ahin/tunnel/1.0.0");

/// 允许接入内网隧道的授权 Peer ID 白名单。
/// 生产环境中应从配置文件或链上授权状态加载，切勿在此直接硬编码正式 Peer ID。
const AUTHORIZED_PEER_IDS: &[&str] = &[
    "12D3KooWExample1AuthorizedGatewayPeerIdForLifePlusPlus",
    "12D3KooWExample2AuthorizedGatewayPeerIdForLifePlusPlus",
];

/// 最大并发隧道数。超出此限制的新请求将被丢弃，防止文件描述符耗尽与 OOM。
const MAX_CONCURRENT_TUNNELS: usize = 10;

/// 等待获取隧道 Semaphore 许可的超时时间（毫秒）。超时则丢弃该入站流。
const TUNNEL_ACQUIRE_TIMEOUT_MS: u64 = 500;

pub async fn run_edge_submarine_daemon(mut swarm: libp2p::Swarm<AhinBehaviour>) {
    println!("⚓ [SUBMARINE] Edge Node initialized. Diving under NAT...");

    let tunnel_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TUNNELS));

    loop {
        match swarm.select_next_some().await {
            // 捕获到来自 L1 网关的穿透请求
            SwarmEvent::IncomingStream {
                mut stream,
                protocol,
                peer_id,
                ..
            } if protocol == AHIN_TUNNEL_PROTOCOL => {
                let peer_id_str = peer_id.to_string();

                // 严格校验对端 Peer ID 是否在授权白名单中，防止未授权节点访问本地硬件 API
                if !AUTHORIZED_PEER_IDS.contains(&peer_id_str.as_str()) {
                    println!(
                        "🚫 [SECURITY] Unauthorized peer {} rejected for tunnel access.",
                        peer_id_str
                    );
                    continue;
                }

                println!(
                    "🚨 [INTRUSION ALARM] Gateway proxy tunnel detected. Routing to local API..."
                );

                let semaphore = Arc::clone(&tunnel_semaphore);
                let peer_id_for_spawn = peer_id_str.clone();
                tokio::spawn(async move {
                    // 尝试获取并发许可；若已达上限或超时，则丢弃此入站流，防止 DDoS 资源耗尽
                    let _permit = match timeout(
                        Duration::from_millis(TUNNEL_ACQUIRE_TIMEOUT_MS),
                        semaphore.acquire_owned(),
                    )
                    .await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(_)) => {
                            println!(
                                "❌ [SEMAPHORE] Semaphore closed, dropping stream from {}.",
                                peer_id_for_spawn
                            );
                            return;
                        }
                        Err(_elapsed) => {
                            println!(
                                "⚠️ [RATE LIMIT] Tunnel limit reached (timeout), dropping stream from {}.",
                                peer_id_for_spawn
                            );
                            return;
                        }
                    };

                    // 1. 在机器人内部，连接到本地回环地址
                    let mut local_stream = match TcpStream::connect(LOCAL_HARDWARE_API).await {
                        Ok(socket) => socket,
                        Err(e) => {
                            println!("❌ [FATAL] Local hardware API offline: {e:?}");
                            return;
                        }
                    };

                    // 2. 桥接 P2P 隧道与本地 HTTP 端口字节流
                    match copy_bidirectional(&mut stream, &mut local_stream).await {
                        Ok((from_p2p, to_p2p)) => {
                            println!(
                                "🔀 [TUNNEL CLOSED] Exchanged {} bytes IN, {} bytes OUT.",
                                from_p2p, to_p2p
                            );
                        }
                        Err(e) => println!("❌ [TUNNEL COLLAPSE] {e:?}"),
                    }
                });
            }
            _ => {}
        }
    }
}
