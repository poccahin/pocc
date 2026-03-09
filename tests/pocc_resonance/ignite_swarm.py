"""
Life++ Planetary Core - Swarm Simulator
Simulates AP2 intent tensors hitting the AHIN gateway from globally distributed edge terminals.
"""

import json
import os
import random
import socket
import time

ROUTER_HOST = os.environ.get("TARGET_ROUTER", "localhost:8000").split(":")[0]
ROUTER_PORT = int(os.environ.get("TARGET_ROUTER", "localhost:8000").split(":")[1])

# 模拟全球边缘终端的 GPS 坐标
EDGE_TERMINALS = [
    {"id": "Term-Tokyo", "gps": [35.6762, 139.6503]},
    {"id": "Term-NY", "gps": [40.7128, -74.0060]},
    {"id": "Term-London", "gps": [51.5074, -0.1278]},
    {"id": "Term-Shenzhen", "gps": [22.5431, 114.0579]},
    {"id": "Term-Berlin", "gps": [52.5200, 13.4050]},
]


def fire_tensor(agent_id, gps, is_malicious=False):
    # 善意意图：平滑的高维张量；恶意意图：极端对抗性噪波
    tensor = [random.uniform(-1.0, 1.0) for _ in range(128)]
    if is_malicious:
        tensor = [9999.0 if i % 2 == 0 else -9999.0 for i in range(128)]
        print(f"💀 [ATTACK] {agent_id} launching adversarial tensor strike from {gps}...")
    else:
        print(f"✅ [SYNC] {agent_id} initiating daily netting consensus from {gps}...")

    payload = {
        "agent_id": agent_id,
        "gps": gps,
        "intent_tensor": tensor,
        "target_semantic_distance": 1.1,
    }

    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect((ROUTER_HOST, ROUTER_PORT))
        s.sendall((json.dumps(payload) + "\n").encode("utf-8"))
        s.recv(1024)
        s.close()
    except Exception as e:
        print(f"Network error: {e}")


if __name__ == "__main__":
    print("🚀 Igniting Life++ Deterministic Swarm Simulator...")
    time.sleep(5)  # 等待 Rust 和 Python 节点完全启动

    tick = 0
    while True:
        # 每秒 2 次合法边缘节点的张量共振 (蓝光)
        terminal = random.choice(EDGE_TERMINALS)
        fire_tensor(terminal["id"], terminal["gps"], is_malicious=False)

        # 每 5 轮注入一次高强度的女巫投毒攻击 (红光大爆破)
        if tick % 5 == 0:
            rogue_gps = [random.uniform(-90, 90), random.uniform(-180, 180)]
            fire_tensor("ROGUE-CARTEL-001", rogue_gps, is_malicious=True)

        tick += 1
        time.sleep(0.5)
