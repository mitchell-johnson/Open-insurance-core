//! Claims workflow management

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use core_kernel::ClaimId;
use crate::ClaimStatus;

/// A workflow task for claims processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    pub id: Uuid,
    pub claim_id: ClaimId,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub assigned_to: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Task types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    ReviewDocuments,
    VerifyCoverage,
    InvestigateClaim,
    ObtainMedicalRecords,
    CalculateReserve,
    Adjudicate,
    ApprovePayment,
    ProcessPayment,
    SendCorrespondence,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    OnHold,
}

impl WorkflowTask {
    /// Creates a new task
    pub fn new(claim_id: ClaimId, task_type: TaskType) -> Self {
        Self {
            id: Uuid::new_v4(),
            claim_id,
            task_type,
            status: TaskStatus::Pending,
            assigned_to: None,
            due_date: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    /// Marks task as completed
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }
}

/// Generates workflow tasks for a claim status change
pub fn generate_tasks_for_status(claim_id: ClaimId, status: ClaimStatus) -> Vec<WorkflowTask> {
    match status {
        ClaimStatus::Fnol => vec![
            WorkflowTask::new(claim_id, TaskType::VerifyCoverage),
            WorkflowTask::new(claim_id, TaskType::ReviewDocuments),
        ],
        ClaimStatus::UnderInvestigation => vec![
            WorkflowTask::new(claim_id, TaskType::InvestigateClaim),
            WorkflowTask::new(claim_id, TaskType::CalculateReserve),
        ],
        ClaimStatus::UnderReview => vec![
            WorkflowTask::new(claim_id, TaskType::Adjudicate),
        ],
        ClaimStatus::Approved => vec![
            WorkflowTask::new(claim_id, TaskType::ApprovePayment),
            WorkflowTask::new(claim_id, TaskType::ProcessPayment),
        ],
        _ => vec![],
    }
}
