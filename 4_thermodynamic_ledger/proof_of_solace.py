"""
L3 Thermodynamic Ledger - Proof of Solace
Quantifies mental entropy reduction into economic value.
"""

import logging
from typing import Dict

logger = logging.getLogger("ledger.proof_of_solace")


class ProofOfSolaceMinter:
    def __init__(self):
        # 心理降熵与物理降熵的汇率 (Base LIFE++ reward per unit of mental stabilization)
        self.SOLACE_EXCHANGE_RATE = 12.5

    def evaluate_mental_entropy_reduction(self, pre_biometrics: Dict, post_biometrics: Dict) -> float:
        """
        通过生物特征 (心率变异性 HRV, 皮质醇估算) 量化心理熵的变化。
        """
        initial_stress = pre_biometrics.get("stress_index", 1.0)
        final_stress = post_biometrics.get("stress_index", 1.0)

        entropy_reduction = initial_stress - final_stress

        if entropy_reduction <= 0:
            logger.debug("No measurable mental entropy reduction detected.")
            return 0.0

        life_plus_reward = entropy_reduction * self.SOLACE_EXCHANGE_RATE
        logger.info(
            "🧘 [Proof of Solace] Human emotional baseline stabilized. Entropy reduced by %.2f.", entropy_reduction
        )
        logger.info("💎 Allocating %.2f LIFE++ to CAI for active emotional buffering.", life_plus_reward)

        return life_plus_reward
