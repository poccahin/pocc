use futures::StreamExt;
use libp2p::swarm::StreamProtocol;
use libp2p::PeerId;
use libp2p_stream::Behaviour as AhinBehaviour;
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio_util::compat::FuturesAsyncReadCompatExt;

/// 机器人的本地控制台或硬件 API 端口 (永远不对外网暴露)
const LOCAL_HARDWARE_API: &str = "127.0.0.1:8080";
const AHIN_TUNNEL_PROTOCOL: StreamProtocol = StreamProtocol::new("/ahin/tunnel/1.0.0");

/// 最大并发隧道数量上限 (DDoS / OOM 防护)
const MAX_CONCURRENT_TUNNELS: usize = 10;

pub async fn run_edge_submarine_daemon(mut swarm: libp2p::Swarm<AhinBehaviour>) {
    println!("⚓ [SUBMARINE] Edge Node initialized. Diving under NAT...");

    // 从 swarm 行为中获取控制句柄并注册入站隧道协议
    let mut control = swarm.behaviour().new_control();
    let mut incoming = control
        .accept(AHIN_TUNNEL_PROTOCOL)
        .expect("AHIN tunnel protocol not already registered");

    // 信号量：严格限制同时开放的隧道数量，防止连接洪泛导致的资源耗尽
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TUNNELS));

    // 在独立任务中驱动 swarm 事件循环（保持连接和协议协商正常运转）
    tokio::spawn(async move {
        loop {
            swarm.select_next_some().await;
        }
    });

    // 处理入站隧道请求
    while let Some((peer_id, stream)) = incoming.next().await {
        // 身份认证存根：校验发起隧道的 Peer 是否为可信节点
        if !is_peer_authorized(&peer_id) {
            println!(
                "🔒 [AUTH] Rejected tunnel request from unauthorized peer: {peer_id}"
            );
            continue;
        }

        // 尝试立即获取信号量许可证；如已达上限则直接拒绝本次请求
        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                println!(
                    "🚫 [OVERLOAD] Max concurrent tunnels ({MAX_CONCURRENT_TUNNELS}) reached. Dropping connection from {peer_id}."
                );
                continue;
            }
        };

        println!(
            "🚨 [INTRUSION ALARM] Gateway proxy tunnel detected. Routing to local API..."
        );

        tokio::spawn(async move {
            // permit 在此 async block 内持有，任务结束时自动释放
            let _permit = permit;

            // 1. 在机器人内部，连接到本地回环地址
            let mut local_stream = match TcpStream::connect(LOCAL_HARDWARE_API).await {
                Ok(socket) => socket,
                Err(e) => {
                    println!("❌ [FATAL] Local hardware API offline: {e:?}");
                    return;
                }
            };

            // 2. 桥接 P2P 隧道与本地 HTTP 端口字节流
            // libp2p::Stream 实现 futures::AsyncRead/Write，需要通过 compat 层适配至 tokio IO
            let mut compat_stream = stream.compat();
            match copy_bidirectional(&mut compat_stream, &mut local_stream).await {
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
}

/// 校验请求隧道的 Peer 是否已授权。
///
/// 生产环境应对接链上白名单（PDA 账户）或 DID Document 签名校验；
/// 当前为存根实现，始终允许来自任意 Peer 的连接（上线前须替换为真实逻辑）。
fn is_peer_authorized(peer_id: &PeerId) -> bool {
    // TODO: 替换为真实的链上白名单查询或 DID 签名验证逻辑
    let _ = peer_id;
    true
}
