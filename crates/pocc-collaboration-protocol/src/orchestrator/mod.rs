//! Composite Task Orchestrator (composite-task-orchestrator)
//!
//! Decomposes a macro-intent (e.g. "clean a 30-floor office building") into a
//! Directed Acyclic Graph (DAG) of micro-tasks, each represented as a standard
//! `CognitiveTransaction` (CTx) that can be dispatched independently to any
//! willing seller agent on the POCC network.
//!
//! ```text
//! MacroIntent (500 USDC)
//!      │
//!      ├─[open_doors]──────────────────────────────────► CTx A
//!      │
//!      ├─[schedule_elevators]──────────────────────────► CTx B
//!      │
//!      ├─[clean_floor_1] (depends: open_doors) ────────► CTx C
//!      │
//!      └─[clean_floor_2] (depends: clean_floor_1) ─────► CTx D
//! ```

use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

use crate::ctx_composer::{
    CognitiveBoundary, CognitiveTransaction, CtxComposer, SettlementInstruction,
};

// ─────────────────────────────────────────────────────────────────────────────
// Error types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Circular dependency detected in sub-tasks")]
    CircularDependency,
    #[error("Insufficient macro budget for decomposition")]
    InsufficientBudget,
    #[error("Sub-task not found: {0}")]
    TaskNotFound(String),
    #[error("Budget fractions must sum to ≤ 1.0 (got {0:.4})")]
    BudgetOverflow(f64),
}

// ─────────────────────────────────────────────────────────────────────────────
// Sub-task status
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SubTaskStatus {
    /// Waiting for upstream dependencies to complete.
    Pending,
    /// All dependencies satisfied — can be dispatched immediately.
    Ready,
    /// Dispatched to a seller; carries the assigned CTx ID.
    Dispatched(String),
    /// L0 executed and L1 consensus confirmed.
    Completed,
    /// Execution failed or L0 safety guardrail triggered.
    Failed(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Sub-task DAG node
// ─────────────────────────────────────────────────────────────────────────────

/// A micro-task node inside the macro-intent DAG.
#[derive(Debug, Clone)]
pub struct SubTaskNode {
    pub id: String,
    pub description: String,
    /// IDs of tasks that must reach `Completed` before this one becomes `Ready`.
    pub dependencies: Vec<String>,
    /// Absolute token amount allocated from the macro budget.
    pub allocated_budget: f64,
    pub status: SubTaskStatus,
}

// ─────────────────────────────────────────────────────────────────────────────
// Composite task (the macro-intent container)
// ─────────────────────────────────────────────────────────────────────────────

/// Holds the full DAG of sub-tasks for one macro-intent.
pub struct CompositeTask {
    pub macro_id: String,
    pub buyer_did: String,
    pub macro_intent: String,
    pub total_budget: f64,
    pub token_symbol: String,
    /// Accumulated fraction of budget allocated to sub-tasks.
    budget_allocated_fraction: f64,
    pub sub_tasks: HashMap<String, SubTaskNode>,
}

impl CompositeTask {
    pub fn new(buyer_did: &str, intent: &str, budget: f64, token: &str) -> Self {
        use sha2::{Digest, Sha256};
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut h = Sha256::new();
        h.update(buyer_did.as_bytes());
        h.update(intent.as_bytes());
        h.update(ts.to_be_bytes());
        let macro_id = hex::encode(h.finalize());

        Self {
            macro_id,
            buyer_did: buyer_did.to_string(),
            macro_intent: intent.to_string(),
            total_budget: budget,
            token_symbol: token.to_string(),
            budget_allocated_fraction: 0.0,
            sub_tasks: HashMap::new(),
        }
    }

    /// Add a sub-task to the DAG.
    ///
    /// `budget_fraction` is the share of `total_budget` assigned to this task
    /// (0.0–1.0).  The sum across all tasks must not exceed 1.0.
    pub fn add_subtask(
        &mut self,
        description: &str,
        dependencies: Vec<String>,
        budget_fraction: f64,
    ) -> Result<String, OrchestratorError> {
        self.budget_allocated_fraction += budget_fraction;
        if self.budget_allocated_fraction > 1.0 + f64::EPSILON {
            return Err(OrchestratorError::BudgetOverflow(
                self.budget_allocated_fraction,
            ));
        }

        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.macro_id.as_bytes());
        h.update(description.as_bytes());
        h.update((self.sub_tasks.len() as u64).to_be_bytes());
        let id = hex::encode(h.finalize());

        let node = SubTaskNode {
            id: id.clone(),
            description: description.to_string(),
            dependencies,
            allocated_budget: self.total_budget * budget_fraction,
            status: SubTaskStatus::Pending,
        };
        self.sub_tasks.insert(id.clone(), node);
        Ok(id)
    }

    /// Validate that the DAG is acyclic using Kahn's algorithm.
    pub fn validate_dag(&self) -> Result<(), OrchestratorError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for (id, node) in &self.sub_tasks {
            in_degree.entry(id.as_str()).or_insert(0);
            for dep in &node.dependencies {
                *in_degree.entry(dep.as_str()).or_insert(0) += 1;
            }
        }
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut visited = 0usize;
        while let Some(id) = queue.pop_front() {
            visited += 1;
            if let Some(node) = self.sub_tasks.get(id) {
                for dep in &node.dependencies {
                    let cnt = in_degree.entry(dep.as_str()).or_insert(0);
                    *cnt = cnt.saturating_sub(1);
                    if *cnt == 0 {
                        queue.push_back(dep.as_str());
                    }
                }
            }
        }
        if visited == self.sub_tasks.len() {
            Ok(())
        } else {
            Err(OrchestratorError::CircularDependency)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Task orchestrator engine
// ─────────────────────────────────────────────────────────────────────────────

pub struct TaskOrchestrator;

impl TaskOrchestrator {
    /// Scan the DAG and promote `Pending` nodes whose dependencies are all
    /// `Completed` to `Ready`.  Returns clones of all `Ready` nodes.
    pub fn get_ready_tasks(task_graph: &mut CompositeTask) -> Vec<SubTaskNode> {
        // Collect the set of completed task IDs first to satisfy the borrow checker.
        let completed: HashSet<String> = task_graph
            .sub_tasks
            .iter()
            .filter(|(_, n)| n.status == SubTaskStatus::Completed)
            .map(|(id, _)| id.clone())
            .collect();

        let mut ready = Vec::new();
        for node in task_graph.sub_tasks.values_mut() {
            if node.status == SubTaskStatus::Pending {
                let all_deps_done = node.dependencies.iter().all(|d| completed.contains(d));
                if all_deps_done {
                    node.status = SubTaskStatus::Ready;
                }
            }
            if node.status == SubTaskStatus::Ready {
                ready.push(node.clone());
            }
        }
        ready
    }

    /// Package a ready sub-task as a dispatchable `CognitiveTransaction`.
    pub fn draft_dispatchable_ctx(
        macro_task: &CompositeTask,
        sub_task: &SubTaskNode,
        seller_did: &str,
    ) -> CognitiveTransaction {
        let boundary = CognitiveBoundary {
            max_compute_units: 5_000,
            max_time_ms: 3_600_000,
            safety_clearance_level: 2,
        };
        let settlement = SettlementInstruction {
            amount: sub_task.allocated_budget,
            token_symbol: macro_task.token_symbol.clone(),
            buyer_signature: format!("sig:{}:{}", macro_task.buyer_did, sub_task.id),
        };
        CtxComposer::draft_transaction(
            &macro_task.buyer_did,
            seller_did,
            &sub_task.description,
            boundary,
            settlement,
        )
    }

    /// Mark a sub-task as completed (called after L1 consensus confirmation).
    pub fn mark_task_completed(
        task_graph: &mut CompositeTask,
        task_id: &str,
    ) -> Result<(), OrchestratorError> {
        task_graph
            .sub_tasks
            .get_mut(task_id)
            .ok_or_else(|| OrchestratorError::TaskNotFound(task_id.to_string()))
            .map(|node| node.status = SubTaskStatus::Completed)
    }

    /// Mark a sub-task as failed with a reason string.
    pub fn mark_task_failed(
        task_graph: &mut CompositeTask,
        task_id: &str,
        reason: &str,
    ) -> Result<(), OrchestratorError> {
        task_graph
            .sub_tasks
            .get_mut(task_id)
            .ok_or_else(|| OrchestratorError::TaskNotFound(task_id.to_string()))
            .map(|node| node.status = SubTaskStatus::Failed(reason.to_string()))
    }

    /// Check whether the entire macro-intent has been fulfilled.
    pub fn is_complete(task_graph: &CompositeTask) -> bool {
        task_graph
            .sub_tasks
            .values()
            .all(|n| n.status == SubTaskStatus::Completed)
    }

    /// Return the total amount still outstanding (dispatched or pending tasks).
    pub fn outstanding_budget(task_graph: &CompositeTask) -> f64 {
        task_graph
            .sub_tasks
            .values()
            .filter(|n| !matches!(n.status, SubTaskStatus::Completed | SubTaskStatus::Failed(_)))
            .map(|n| n.allocated_budget)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_building_dag() -> (CompositeTask, String, String, String) {
        let mut task = CompositeTask::new(
            "did:buyer:enterprise",
            "Clean 3-floor office",
            300.0,
            "USDC",
        );
        let open_doors = task.add_subtask("Open all doors", vec![], 0.1).unwrap();
        let floor1 = task
            .add_subtask("Clean floor 1", vec![open_doors.clone()], 0.3)
            .unwrap();
        let floor2 = task
            .add_subtask("Clean floor 2", vec![floor1.clone()], 0.3)
            .unwrap();
        (task, open_doors, floor1, floor2)
    }

    #[test]
    fn dag_validation_passes_for_valid_dag() {
        let (task, _, _, _) = build_building_dag();
        assert!(task.validate_dag().is_ok());
    }

    #[test]
    fn initial_ready_tasks_are_those_with_no_deps() {
        let (mut task, open_doors, _, _) = build_building_dag();
        let ready = TaskOrchestrator::get_ready_tasks(&mut task);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, open_doors);
    }

    #[test]
    fn completing_task_unlocks_dependents() {
        let (mut task, open_doors, floor1, _) = build_building_dag();
        TaskOrchestrator::mark_task_completed(&mut task, &open_doors).unwrap();
        let ready = TaskOrchestrator::get_ready_tasks(&mut task);
        let ready_ids: Vec<_> = ready.iter().map(|n| n.id.as_str()).collect();
        assert!(ready_ids.contains(&floor1.as_str()));
    }

    #[test]
    fn is_complete_only_when_all_done() {
        let (mut task, open_doors, floor1, floor2) = build_building_dag();
        assert!(!TaskOrchestrator::is_complete(&task));
        TaskOrchestrator::mark_task_completed(&mut task, &open_doors).unwrap();
        TaskOrchestrator::mark_task_completed(&mut task, &floor1).unwrap();
        TaskOrchestrator::mark_task_completed(&mut task, &floor2).unwrap();
        assert!(TaskOrchestrator::is_complete(&task));
    }

    #[test]
    fn budget_overflow_is_rejected() {
        let mut task = CompositeTask::new("did:buyer", "intent", 100.0, "USDC");
        task.add_subtask("task1", vec![], 0.6).unwrap();
        let result = task.add_subtask("task2", vec![], 0.5);
        assert!(matches!(result, Err(OrchestratorError::BudgetOverflow(_))));
    }

    #[test]
    fn draft_ctx_has_correct_budget_allocation() {
        let (mut task, open_doors_id, _, _) = build_building_dag();
        TaskOrchestrator::get_ready_tasks(&mut task);
        let node = task.sub_tasks[&open_doors_id].clone();
        let ctx = TaskOrchestrator::draft_dispatchable_ctx(&task, &node, "did:robot:001");
        assert!((ctx.settlement.amount - 30.0).abs() < f64::EPSILON); // 10% of 300
    }
}

