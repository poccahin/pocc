use pocc_collaboration_protocol::ctx_composer::CognitiveTransaction;

pub struct DailyNettingProcessor {
    pub token_symbol: String,
    pending_transactions: Vec<CognitiveTransaction>,
}
