use anchor_lang::prelude::*;

#[account]
pub struct AgentPersona {
    pub genesis_timestamp: i64,
    pub total_tasks_attempted: u64,
    pub total_valid_pocc: u64,
    pub total_value_settled: u64,
    pub staked_life_plus: u64,
    pub is_slashed: bool,
    pub last_active_timestamp: i64,
}

#[account]
pub struct AuditorWhitelist {
    pub is_active: bool,
    pub authority: Pubkey,
    pub total_slashes_executed: u64,
}

impl AgentPersona {
    pub fn calculate_scog_score(&self, now_timestamp: i64) -> u64 {
        if self.is_slashed {
            return 0;
        }

        let age_seconds = now_timestamp.saturating_sub(self.genesis_timestamp);
        let age_factor_days = (age_seconds as u64) / 86_400;

        let alignment_ratio = if self.total_tasks_attempted == 0 {
            0
        } else {
            self.total_valid_pocc.saturating_mul(100) / self.total_tasks_attempted
        };

        let base_score = age_factor_days.saturating_mul(alignment_ratio);
        let volume_bonus = if self.total_value_settled == 0 {
            0
        } else {
            (self.total_value_settled as f64).log10() as u64
        };

        base_score.saturating_add(volume_bonus)
    }
}

#[account]
pub struct InteractionEdge {
    pub orchestrator: Pubkey,
    pub worker: Pubkey,
    pub interaction_count: u64,
}

#[account]
pub struct AhinTimeline {
    pub current_global_hash: [u8; 32],
}

#[account]
pub struct CompositeTaskState {
    pub orchestrator: Pubkey,
    pub total_bounty: u64,
    pub subtask_rewards: Vec<SubtaskReward>,
    pub completed_subtasks: Vec<u64>,
    pub settled_subtasks: Vec<u64>,
    pub last_processed_index: u32,
    pub is_fully_settled: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct SubtaskReward {
    pub subtask_id: u64,
    pub payment_bips: u16,
    pub worker: Pubkey,
}
