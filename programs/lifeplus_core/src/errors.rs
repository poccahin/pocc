use anchor_lang::prelude::*;

#[error_code]
pub enum LifePlusError {
    #[msg("Thermodynamic constraint violated: Compute limit exceeded the on-chain boundary.")]
    ThermodynamicBoundaryExceeded,
    #[msg("Zero-Knowledge Cognitive Proof verification failed.")]
    InvalidCognitiveProof,

    #[msg("The task intent state is no longer active.")]
    TaskAlreadyClosed,
    #[msg("Intent hash does not match the task intent state.")]
    TaskIntentHashMismatch,
    #[msg("Subtask has not passed PoCC verification yet.")]
    SubtaskNotVerified,
    #[msg("This CTx settlement has already been claimed.")]
    DoubleSpendingAttempt,
    #[msg("Agent identity is already marked as slashed/dead.")]
    AgentAlreadyDead,
    #[msg("Arithmetic overflow while processing protocol math.")]
    ArithmeticOverflow,
    #[msg("Invalid payment ratio in bips.")]
    InvalidPaymentBips,
    #[msg("Settlement currently locked to prevent reentrancy-like double execution.")]
    SettlementLocked,
    #[msg("Unauthorized settlement authority.")]
    UnauthorizedAuthority,
    #[msg("Escrow and worker token mints must match.")]
    MintMismatch,
}
