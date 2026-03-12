use futures::StreamExt;
use libp2p::swarm::{StreamProtocol, SwarmEvent};
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;

use crate::AhinBehaviour;

/// 机器人的本地控制台或硬件 API 端口 (永远不对外网暴露)
const LOCAL_HARDWARE_API: &str = "127.0.0.1:8080";
const AHIN_TUNNEL_PROTOCOL: StreamProtocol = StreamProtocol::new("/ahin/tunnel/1.0.0");

pub async fn run_edge_submarine_daemon(mut swarm: libp2p::Swarm<AhinBehaviour>) {
    println!("⚓ [SUBMARINE] Edge Node initialized. Diving under NAT...");

    loop {
        match swarm.select_next_some().await {
            // 捕获到来自 L1 网关的穿透请求
            SwarmEvent::IncomingStream {
                mut stream,
                protocol,
                ..
            } if protocol == AHIN_TUNNEL_PROTOCOL => {
                println!(
                    "🚨 [INTRUSION ALARM] Gateway proxy tunnel detected. Routing to local API..."
                );

                tokio::spawn(async move {
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
