import json
import random
import socket


def send_tensor(agent_id, tensor_values):
    payload = {
        "agent_id": agent_id,
        "intent_tensor": tensor_values,
        "target_semantic_distance": 1.1,
    }
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(("127.0.0.1", 8000))
    s.sendall((json.dumps(payload) + "\n").encode("utf-8"))
    response = s.recv(1024).decode("utf-8")
    print(f"[{agent_id}] Response: {response.strip()}")
    s.close()


print("🚀 Launching concurrent tensor requests to AHIN Daemon...")

# 模拟发送 5 个正常请求和 1 个被构造的极端噪声请求
for i in range(5):
    # 模拟正常张量
    send_tensor(f"Normal-CAI-{i}", [random.uniform(-1.0, 1.0) for _ in range(128)])

# 模拟具有极高数值的"投毒"对抗性张量
send_tensor("Hacker-Cartel", [9999.0 if i % 2 == 0 else -9999.0 for i in range(128)])
