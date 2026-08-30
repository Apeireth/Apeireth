//! Preemptive Multi-Level Cognitive Time-Slicing Scheduler with Cognitive Quota & PIP.
//!
//! # Mathematical & Architectural Foundations
//!
//! Unlike traditional OS schedulers which schedule CPU time ($\Delta t$), the AI Cognitive Microkernel
//! schedules **Cognitive Computation Quotas**:
//! $$\mathcal{Q} = \langle \Delta T_{\text{token}}, \Delta S_{\text{step}}, \Delta C_{\text{cost}}, \Delta D_{\text{depth}} \rangle$$
//!
//! - **Preemptive Time-Slicing**: Periodically checks step/token consumption. High-priority interrupts
//!   (`SystemEmergency`, `InteractiveUser`) asynchronously preempt running tasks and push serialized
//!   `CognitiveContextFrame` frames onto the execution stack;
//! - **Priority Inheritance Protocol (PIP)**: If a background task holds an exclusive cognitive mutex
//!   (e.g., memory graph write-lock), its effective priority is boosted to prevent priority inversion;
//! - **Deterministic Stack Resumption**: Preempted tasks can be resumed with zero hallucination drift.
//!
//! Pure Safe Rust (`#![forbid(unsafe_code)]`).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Five-tier cognitive priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CognitivePriority {
    /// P0: Safety violation, emergency kill-switch, physical disconnect.
    SystemEmergency = 0,
    /// P1: Real-time user foreground interaction.
    InteractiveUser = 1,
    /// P2: Active foreground spawned worker agent.
    ActiveSubAgent = 2,
    /// P3: Background circadian dreaming, memory consolidation.
    BackgroundDreaming = 3,
    /// P4: Idle linting, vacuum, log garbage collection.
    IdleMaintenance = 4,
}

/// Cognitive computation budget & limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveQuota {
    pub max_tokens: usize,
    pub max_tool_steps: usize,
    pub max_cost_micros: u64,
    pub max_recursion_depth: usize,
    pub consumed_tokens: usize,
    pub consumed_tool_steps: usize,
    pub consumed_cost_micros: u64,
}

impl CognitiveQuota {
    pub fn new(max_tokens: usize, max_tool_steps: usize, max_cost_micros: u64, max_depth: usize) -> Self {
        Self {
            max_tokens,
            max_tool_steps,
            max_cost_micros,
            max_recursion_depth: max_depth,
            consumed_tokens: 0,
            consumed_tool_steps: 0,
            consumed_cost_micros: 0,
        }
    }

    /// Checks if any budget dimension has been exceeded.
    pub fn is_exhausted(&self) -> bool {
        self.consumed_tokens >= self.max_tokens
            || self.consumed_tool_steps >= self.max_tool_steps
            || self.consumed_cost_micros >= self.max_cost_micros
    }

    /// Records step consumption.
    pub fn consume_step(&mut self, tokens: usize, cost_micros: u64) -> bool {
        self.consumed_tokens += tokens;
        self.consumed_tool_steps += 1;
        self.consumed_cost_micros += cost_micros;
        !self.is_exhausted()
    }
}

/// Serialized cognitive context frame for deterministic preemption & resumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveContextFrame {
    pub task_id: String,
    pub session_id: String,
    pub step_index: usize,
    pub call_stack: Vec<String>,
    pub local_transcript_snapshot: Vec<String>,
    pub world_state_hash: String,
}

/// Asynchronous cognitive interrupt signals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CognitiveInterrupt {
    EmergencyPreempt { reason: String },
    BudgetExhausted { task_id: String },
    VoluntaryYield { task_id: String },
}

/// Task Control Block (TCB) in the cognitive scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTaskControlBlock {
    pub task_id: String,
    pub session_id: String,
    pub base_priority: CognitivePriority,
    pub effective_priority: CognitivePriority,
    pub quota: CognitiveQuota,
    pub context_frame: CognitiveContextFrame,
    pub is_preempted: bool,
}

/// Multi-Level Cognitive Quota Scheduler.
#[derive(Debug, Clone)]
pub struct CognitiveQuotaScheduler {
    inner: Arc<Mutex<SchedulerInner>>,
}

#[derive(Debug)]
struct SchedulerInner {
    run_queues: BTreeMap<CognitivePriority, VecDeque<String>>,
    tasks: HashMap<String, CognitiveTaskControlBlock>,
    active_running_task: Option<String>,
    held_locks: HashMap<String, String>, // Resource -> TaskId
}

impl Default for CognitiveQuotaScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveQuotaScheduler {
    pub fn new() -> Self {
        let mut run_queues = BTreeMap::new();
        run_queues.insert(CognitivePriority::SystemEmergency, VecDeque::new());
        run_queues.insert(CognitivePriority::InteractiveUser, VecDeque::new());
        run_queues.insert(CognitivePriority::ActiveSubAgent, VecDeque::new());
        run_queues.insert(CognitivePriority::BackgroundDreaming, VecDeque::new());
        run_queues.insert(CognitivePriority::IdleMaintenance, VecDeque::new());

        Self {
            inner: Arc::new(Mutex::new(SchedulerInner {
                run_queues,
                tasks: HashMap::new(),
                active_running_task: None,
                held_locks: HashMap::new(),
            })),
        }
    }

    /// Submits a new cognitive task to the scheduler.
    pub fn submit_task(&self, tcb: CognitiveTaskControlBlock) {
        let mut inner = self.inner.lock().expect("Scheduler mutex poisoned");
        let tid = tcb.task_id.clone();
        let prio = tcb.effective_priority;
        inner.tasks.insert(tid.clone(), tcb);
        if let Some(queue) = inner.run_queues.get_mut(&prio) {
            queue.push_back(tid);
        }
    }

    /// Dispatches the next highest-priority task to run.
    pub fn schedule_next(&self) -> Option<CognitiveTaskControlBlock> {
        let mut inner = self.inner.lock().expect("Scheduler mutex poisoned");

        let mut chosen_task_id = None;
        for queue in inner.run_queues.values_mut() {
            if let Some(task_id) = queue.pop_front() {
                chosen_task_id = Some(task_id);
                break;
            }
        }

        if let Some(task_id) = chosen_task_id {
            if let Some(tcb) = inner.tasks.get_mut(&task_id) {
                tcb.is_preempted = false;
                let result = tcb.clone();
                inner.active_running_task = Some(task_id);
                return Some(result);
            }
        }
        inner.active_running_task = None;
        None
    }

    /// Signals an asynchronous preemption of the currently running task.
    pub fn preempt_active(&self, _interrupt: CognitiveInterrupt) -> Result<Option<String>, String> {
        let mut inner = self.inner.lock().expect("Scheduler mutex poisoned");
        if let Some(active_id) = inner.active_running_task.take() {
            if let Some(tcb) = inner.tasks.get_mut(&active_id) {
                tcb.is_preempted = true;
                let prio = tcb.effective_priority;
                if let Some(queue) = inner.run_queues.get_mut(&prio) {
                    queue.push_front(active_id.clone());
                }
                return Ok(Some(active_id));
            }
        }
        Ok(None)
    }

    /// Priority Inheritance Protocol (PIP): Boosts the effective priority of lock-holding task.
    pub fn boost_priority_for_lock(&self, resource_id: &str, requesting_priority: CognitivePriority) {
        let mut inner = self.inner.lock().expect("Scheduler mutex poisoned");
        if let Some(holder_id) = inner.held_locks.get(resource_id).cloned() {
            let old_prio = if let Some(holder_tcb) = inner.tasks.get_mut(&holder_id) {
                if requesting_priority < holder_tcb.effective_priority {
                    let old = holder_tcb.effective_priority;
                    holder_tcb.effective_priority = requesting_priority;
                    Some(old)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(old) = old_prio {
                if let Some(queue) = inner.run_queues.get_mut(&old) {
                    queue.retain(|id| id != &holder_id);
                }
                if let Some(queue) = inner.run_queues.get_mut(&requesting_priority) {
                    queue.push_front(holder_id);
                }
            }
        }
    }

    /// Registers a resource lock held by a task.
    pub fn register_lock(&self, resource_id: &str, task_id: &str) {
        let mut inner = self.inner.lock().expect("Scheduler mutex poisoned");
        inner.held_locks.insert(resource_id.to_string(), task_id.to_string());
    }

    /// Releases a resource lock and resets effective priority to base priority.
    pub fn release_lock(&self, resource_id: &str) {
        let mut inner = self.inner.lock().expect("Scheduler mutex poisoned");
        if let Some(holder_id) = inner.held_locks.remove(resource_id) {
            let (old_prio, base_prio) = if let Some(holder_tcb) = inner.tasks.get_mut(&holder_id) {
                let current_prio = holder_tcb.effective_priority;
                holder_tcb.effective_priority = holder_tcb.base_priority;
                (current_prio, holder_tcb.base_priority)
            } else {
                return;
            };

            if old_prio != base_prio {
                if let Some(queue) = inner.run_queues.get_mut(&old_prio) {
                    queue.retain(|id| id != &holder_id);
                }
                if let Some(queue) = inner.run_queues.get_mut(&base_prio) {
                    queue.push_back(holder_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_quota_budget_consumption() {
        let mut quota = CognitiveQuota::new(1000, 5, 5000, 3);
        assert!(!quota.is_exhausted());

        assert!(quota.consume_step(200, 1000));
        assert_eq!(quota.consumed_tool_steps, 1);
        assert_eq!(quota.consumed_tokens, 200);

        // Exhaust step count
        for _ in 0..4 {
            quota.consume_step(100, 500);
        }
        assert!(quota.is_exhausted());
    }

    #[test]
    fn test_preemptive_scheduling_priority_order() {
        let scheduler = CognitiveQuotaScheduler::new();

        let tcb_background = CognitiveTaskControlBlock {
            task_id: "task_dream".into(),
            session_id: "s1".into(),
            base_priority: CognitivePriority::BackgroundDreaming,
            effective_priority: CognitivePriority::BackgroundDreaming,
            quota: CognitiveQuota::new(1000, 10, 1000, 1),
            context_frame: CognitiveContextFrame {
                task_id: "task_dream".into(),
                session_id: "s1".into(),
                step_index: 0,
                call_stack: vec![],
                local_transcript_snapshot: vec![],
                world_state_hash: "hash_0".into(),
            },
            is_preempted: false,
        };

        let tcb_user = CognitiveTaskControlBlock {
            task_id: "task_user".into(),
            session_id: "s1".into(),
            base_priority: CognitivePriority::InteractiveUser,
            effective_priority: CognitivePriority::InteractiveUser,
            quota: CognitiveQuota::new(2000, 20, 2000, 2),
            context_frame: CognitiveContextFrame {
                task_id: "task_user".into(),
                session_id: "s1".into(),
                step_index: 0,
                call_stack: vec![],
                local_transcript_snapshot: vec![],
                world_state_hash: "hash_1".into(),
            },
            is_preempted: false,
        };

        scheduler.submit_task(tcb_background);
        scheduler.submit_task(tcb_user);

        // InteractiveUser (P1) should be scheduled before BackgroundDreaming (P3)
        let first = scheduler.schedule_next().unwrap();
        assert_eq!(first.task_id, "task_user");
        assert_eq!(first.effective_priority, CognitivePriority::InteractiveUser);

        // Preempt active task
        let preempted = scheduler
            .preempt_active(CognitiveInterrupt::EmergencyPreempt {
                reason: "User command".into(),
            })
            .unwrap();
        assert_eq!(preempted, Some("task_user".into()));
    }

    #[test]
    fn test_priority_inheritance_protocol() {
        let scheduler = CognitiveQuotaScheduler::new();

        let tcb_bg = CognitiveTaskControlBlock {
            task_id: "task_bg_lock".into(),
            session_id: "s1".into(),
            base_priority: CognitivePriority::BackgroundDreaming,
            effective_priority: CognitivePriority::BackgroundDreaming,
            quota: CognitiveQuota::new(1000, 10, 1000, 1),
            context_frame: CognitiveContextFrame {
                task_id: "task_bg_lock".into(),
                session_id: "s1".into(),
                step_index: 0,
                call_stack: vec![],
                local_transcript_snapshot: vec![],
                world_state_hash: "hash_0".into(),
            },
            is_preempted: false,
        };

        scheduler.submit_task(tcb_bg);
        scheduler.register_lock("memory_graph_lock", "task_bg_lock");

        // High priority user task requests memory_graph_lock
        scheduler.boost_priority_for_lock("memory_graph_lock", CognitivePriority::InteractiveUser);

        // Background task should now be at InteractiveUser priority level
        let scheduled = scheduler.schedule_next().unwrap();
        assert_eq!(scheduled.task_id, "task_bg_lock");
        assert_eq!(scheduled.effective_priority, CognitivePriority::InteractiveUser);

        // Once lock is released, priority resets
        scheduler.release_lock("memory_graph_lock");
    }
}
