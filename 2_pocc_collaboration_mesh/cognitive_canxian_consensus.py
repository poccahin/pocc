"""Cognitive Canxian Consensus Engine.

在 AP2 协议中，Worker 智能体使用该引擎评估外部意图是否落入自身认知坎陷，
并以「认知摩擦」作为是否建立协作契约的判据。
"""

from __future__ import annotations

import math

try:
    import mlx.core as mx  # type: ignore

    BACKEND = "mlx"

    def _array(values):
        return mx.array(values)

    def _norm(values):
        return float(mx.linalg.norm(values))

    def _dot(a, b):
        return float(mx.dot(a, b))

except Exception:  # pragma: no cover - fallback for environments without MLX
    BACKEND = "python"

    def _array(values):
        return [float(v) for v in values]

    def _norm(values):
        return math.sqrt(sum(v * v for v in values))

    def _dot(a, b):
        return sum(x * y for x, y in zip(a, b))


class CognitiveCanxianEvaluator:
    """认知坎陷共识评估器。"""

    def __init__(self, agent_capability_profile: list[float]):
        print(f"🧠 [CANXIAN] Initializing Agent's Cognitive Attractor Basin... (backend={BACKEND})")

        if not agent_capability_profile:
            raise ValueError("agent_capability_profile cannot be empty")

        self.self_capability_tensor = self._normalize(_array(agent_capability_profile))

    def evaluate_intent_friction(self, intent_vector: list[float], max_friction_allowed: float) -> bool:
        """计算外部意图与自身认知坎陷的语义摩擦。"""
        if max_friction_allowed < 0:
            raise ValueError("max_friction_allowed must be >= 0")

        incoming_intent = self._normalize(_array(intent_vector))

        semantic_alignment = _dot(self.self_capability_tensor, incoming_intent)
        cognitive_friction = 1.0 - semantic_alignment

        print(f"📐 [AHIN] Semantic Alignment: {semantic_alignment:.4f}")
        print(f"🔥 [CANXIAN] Cognitive Friction: {cognitive_friction:.4f} (Threshold: {max_friction_allowed})")

        if cognitive_friction <= max_friction_allowed:
            print("✅ [CONSENSUS REACHED] Intent has fallen into the Cognitive Canxian. Handshake accepted.")
            return True

        print("❌ [CONSENSUS FAILED] Friction too high. Intent bounced off the cognitive perimeter.")
        return False

    @staticmethod
    def _normalize(vector):
        norm = _norm(vector)
        if norm == 0.0:
            raise ValueError("zero vector cannot be normalized")

        if isinstance(vector, list):
            return [v / norm for v in vector]

        return vector / norm


if __name__ == "__main__":
    worker_capability = [0.9, 0.8, 0.1, 0.05, 0.2]
    canxian_evaluator = CognitiveCanxianEvaluator(worker_capability)

    incoming_intent_a = [0.85, 0.82, 0.15, 0.01, 0.1]
    print("\n--- 评估意图 A (高匹配度) ---")
    canxian_evaluator.evaluate_intent_friction(incoming_intent_a, max_friction_allowed=0.1)

    incoming_intent_b = [0.05, 0.1, 0.9, 0.88, 0.95]
    print("\n--- 评估意图 B (低匹配度) ---")
    canxian_evaluator.evaluate_intent_friction(incoming_intent_b, max_friction_allowed=0.1)
