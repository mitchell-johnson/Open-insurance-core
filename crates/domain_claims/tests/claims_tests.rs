//! Comprehensive tests for domain_claims

use chrono::{NaiveDate, Utc, Days};
use rust_decimal_macros::dec;

use core_kernel::{ClaimId, PolicyId, PartyId, Money, Currency};

use domain_claims::claim::{Claim, ClaimStatus, LossType};
use domain_claims::adjudication::{AdjudicationResult, AdjudicationDecision, AdjudicationReason};
use domain_claims::workflow::{WorkflowTask, TaskType, TaskStatus, generate_tasks_for_status};
use domain_claims::reserve::{Reserve, ReserveType};
use domain_claims::payment::{ClaimPayment, PaymentType, PaymentMethod};

// ============================================================================
// Claim Tests
// ============================================================================

mod claim_tests {
    use super::*;

    fn create_test_claim() -> Claim {
        let policy_id = PolicyId::new_v7();
        let claimant_id = PartyId::new_v7();
        let loss_date = Utc::now().date_naive() - Days::new(5);

        Claim::fnol(policy_id, claimant_id, loss_date, LossType::Accident, Currency::USD)
    }

    #[test]
    fn test_claim_fnol() {
        let claim = create_test_claim();

        assert_eq!(claim.status, ClaimStatus::Fnol);
        assert_eq!(claim.loss_type, LossType::Accident);
        assert!(claim.claim_number.starts_with("CLM-"));
        assert!(claim.reserves.is_empty());
        assert!(claim.payments.is_empty());
        assert_eq!(claim.paid_amount.amount(), dec!(0));
    }

    #[test]
    fn test_claim_update_status_valid_transition() {
        let mut claim = create_test_claim();

        // FNOL -> UnderInvestigation is valid
        let result = claim.update_status(ClaimStatus::UnderInvestigation);
        assert!(result.is_ok());
        assert_eq!(claim.status, ClaimStatus::UnderInvestigation);
    }

    #[test]
    fn test_claim_update_status_invalid_transition() {
        let mut claim = create_test_claim();

        // FNOL -> Approved is invalid
        let result = claim.update_status(ClaimStatus::Approved);
        assert!(result.is_err());
    }

    #[test]
    fn test_claim_status_transitions_fnol_to_pending_documentation() {
        let mut claim = create_test_claim();
        assert!(claim.update_status(ClaimStatus::PendingDocumentation).is_ok());
    }

    #[test]
    fn test_claim_status_transitions_under_investigation_to_under_review() {
        let mut claim = create_test_claim();
        claim.update_status(ClaimStatus::UnderInvestigation).unwrap();
        assert!(claim.update_status(ClaimStatus::UnderReview).is_ok());
    }

    #[test]
    fn test_claim_status_transitions_under_review_to_approved() {
        let mut claim = create_test_claim();
        claim.update_status(ClaimStatus::UnderInvestigation).unwrap();
        claim.update_status(ClaimStatus::UnderReview).unwrap();
        assert!(claim.update_status(ClaimStatus::Approved).is_ok());
    }

    #[test]
    fn test_claim_status_transitions_under_review_to_partially_approved() {
        let mut claim = create_test_claim();
        claim.update_status(ClaimStatus::UnderInvestigation).unwrap();
        claim.update_status(ClaimStatus::UnderReview).unwrap();
        assert!(claim.update_status(ClaimStatus::PartiallyApproved).is_ok());
    }

    #[test]
    fn test_claim_status_transitions_under_review_to_denied() {
        let mut claim = create_test_claim();
        claim.update_status(ClaimStatus::UnderInvestigation).unwrap();
        claim.update_status(ClaimStatus::UnderReview).unwrap();
        assert!(claim.update_status(ClaimStatus::Denied).is_ok());
    }

    #[test]
    fn test_claim_status_transitions_approved_to_closed() {
        let mut claim = create_test_claim();
        claim.update_status(ClaimStatus::UnderInvestigation).unwrap();
        claim.update_status(ClaimStatus::UnderReview).unwrap();
        claim.update_status(ClaimStatus::Approved).unwrap();
        assert!(claim.update_status(ClaimStatus::Closed).is_ok());
    }

    #[test]
    fn test_claim_status_transitions_closed_to_reopened() {
        let mut claim = create_test_claim();
        claim.update_status(ClaimStatus::UnderInvestigation).unwrap();
        claim.update_status(ClaimStatus::UnderReview).unwrap();
        claim.update_status(ClaimStatus::Approved).unwrap();
        claim.update_status(ClaimStatus::Closed).unwrap();
        assert!(claim.update_status(ClaimStatus::Reopened).is_ok());
    }

    #[test]
    fn test_claim_status_transitions_any_to_withdrawn() {
        let mut claim = create_test_claim();
        assert!(claim.update_status(ClaimStatus::Withdrawn).is_ok());
    }

    #[test]
    fn test_claim_add_reserve() {
        let mut claim = create_test_claim();
        let reserve = Reserve::new(claim.id, ReserveType::CaseReserve, Money::new(dec!(10000), Currency::USD));

        claim.add_reserve(reserve);

        assert_eq!(claim.reserves.len(), 1);
    }

    #[test]
    fn test_claim_add_payment() {
        let mut claim = create_test_claim();
        let payment = ClaimPayment::new(
            claim.id,
            PartyId::new_v7(),
            Money::new(dec!(5000), Currency::USD),
            PaymentType::Indemnity,
            PaymentMethod::BankTransfer,
        );

        claim.add_payment(payment);

        assert_eq!(claim.payments.len(), 1);
        assert_eq!(claim.paid_amount.amount(), dec!(5000));
    }

    #[test]
    fn test_claim_total_reserve() {
        let mut claim = create_test_claim();
        claim.add_reserve(Reserve::new(claim.id, ReserveType::CaseReserve, Money::new(dec!(10000), Currency::USD)));
        claim.add_reserve(Reserve::new(claim.id, ReserveType::LegalExpense, Money::new(dec!(2000), Currency::USD)));

        let total = claim.total_reserve();
        assert_eq!(total.amount(), dec!(12000));
    }

    #[test]
    fn test_all_loss_types() {
        let types = vec![
            LossType::Death,
            LossType::Disability,
            LossType::CriticalIllness,
            LossType::Hospitalization,
            LossType::Accident,
            LossType::Property,
            LossType::Liability,
            LossType::Other,
        ];

        for loss_type in types {
            let json = serde_json::to_string(&loss_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_claim_statuses() {
        let statuses = vec![
            ClaimStatus::Fnol,
            ClaimStatus::UnderInvestigation,
            ClaimStatus::PendingDocumentation,
            ClaimStatus::UnderReview,
            ClaimStatus::Approved,
            ClaimStatus::PartiallyApproved,
            ClaimStatus::Denied,
            ClaimStatus::Closed,
            ClaimStatus::Withdrawn,
            ClaimStatus::Reopened,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Adjudication Tests
// ============================================================================

mod adjudication_tests {
    use super::*;

    #[test]
    fn test_adjudication_result_approve() {
        let claim_id = ClaimId::new_v7();
        let amount = Money::new(dec!(10000), Currency::USD);

        let result = AdjudicationResult::approve(claim_id, amount, "adjuster@example.com");

        assert_eq!(result.decision, AdjudicationDecision::Approved);
        assert_eq!(result.approved_amount.unwrap().amount(), dec!(10000));
        assert!(result.reasons.contains(&AdjudicationReason::Covered));
    }

    #[test]
    fn test_adjudication_result_deny() {
        let claim_id = ClaimId::new_v7();

        let result = AdjudicationResult::deny(claim_id, AdjudicationReason::PolicyNotInForce, "adjuster@example.com");

        assert_eq!(result.decision, AdjudicationDecision::Denied);
        assert!(result.approved_amount.is_none());
        assert!(result.reasons.contains(&AdjudicationReason::PolicyNotInForce));
    }

    #[test]
    fn test_all_adjudication_decisions() {
        let decisions = vec![
            AdjudicationDecision::Approved,
            AdjudicationDecision::PartiallyApproved,
            AdjudicationDecision::Denied,
            AdjudicationDecision::PendingInformation,
            AdjudicationDecision::Referred,
        ];

        for decision in decisions {
            let json = serde_json::to_string(&decision).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_adjudication_reasons() {
        let reasons = vec![
            AdjudicationReason::Covered,
            AdjudicationReason::Exclusion("Test exclusion".to_string()),
            AdjudicationReason::WaitingPeriod,
            AdjudicationReason::PolicyNotInForce,
            AdjudicationReason::ExceedsLimit,
            AdjudicationReason::FraudSuspected,
            AdjudicationReason::InsufficientDocumentation,
            AdjudicationReason::PreExistingCondition,
            AdjudicationReason::Other("Other reason".to_string()),
        ];

        for reason in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Workflow Tests
// ============================================================================

mod workflow_tests {
    use super::*;

    #[test]
    fn test_workflow_task_new() {
        let claim_id = ClaimId::new_v7();
        let task = WorkflowTask::new(claim_id, TaskType::ReviewDocuments);

        assert_eq!(task.claim_id, claim_id);
        assert_eq!(task.task_type, TaskType::ReviewDocuments);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.assigned_to.is_none());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_workflow_task_complete() {
        let claim_id = ClaimId::new_v7();
        let mut task = WorkflowTask::new(claim_id, TaskType::ReviewDocuments);

        task.complete();

        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_generate_tasks_for_fnol() {
        let claim_id = ClaimId::new_v7();
        let tasks = generate_tasks_for_status(claim_id, ClaimStatus::Fnol);

        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().any(|t| t.task_type == TaskType::VerifyCoverage));
        assert!(tasks.iter().any(|t| t.task_type == TaskType::ReviewDocuments));
    }

    #[test]
    fn test_generate_tasks_for_under_investigation() {
        let claim_id = ClaimId::new_v7();
        let tasks = generate_tasks_for_status(claim_id, ClaimStatus::UnderInvestigation);

        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().any(|t| t.task_type == TaskType::InvestigateClaim));
        assert!(tasks.iter().any(|t| t.task_type == TaskType::CalculateReserve));
    }

    #[test]
    fn test_generate_tasks_for_under_review() {
        let claim_id = ClaimId::new_v7();
        let tasks = generate_tasks_for_status(claim_id, ClaimStatus::UnderReview);

        assert_eq!(tasks.len(), 1);
        assert!(tasks.iter().any(|t| t.task_type == TaskType::Adjudicate));
    }

    #[test]
    fn test_generate_tasks_for_approved() {
        let claim_id = ClaimId::new_v7();
        let tasks = generate_tasks_for_status(claim_id, ClaimStatus::Approved);

        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().any(|t| t.task_type == TaskType::ApprovePayment));
        assert!(tasks.iter().any(|t| t.task_type == TaskType::ProcessPayment));
    }

    #[test]
    fn test_generate_tasks_for_other_status() {
        let claim_id = ClaimId::new_v7();
        let tasks = generate_tasks_for_status(claim_id, ClaimStatus::Closed);

        assert!(tasks.is_empty());
    }

    #[test]
    fn test_all_task_types() {
        let types = vec![
            TaskType::ReviewDocuments,
            TaskType::VerifyCoverage,
            TaskType::InvestigateClaim,
            TaskType::ObtainMedicalRecords,
            TaskType::CalculateReserve,
            TaskType::Adjudicate,
            TaskType::ApprovePayment,
            TaskType::ProcessPayment,
            TaskType::SendCorrespondence,
        ];

        for task_type in types {
            let json = serde_json::to_string(&task_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_task_statuses() {
        let statuses = vec![
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::Cancelled,
            TaskStatus::OnHold,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Reserve Tests
// ============================================================================

mod reserve_tests {
    use super::*;

    #[test]
    fn test_reserve_new() {
        let claim_id = ClaimId::new_v7();
        let reserve = Reserve::new(claim_id, ReserveType::CaseReserve, Money::new(dec!(10000), Currency::USD));

        assert_eq!(reserve.claim_id, claim_id);
        assert_eq!(reserve.reserve_type, ReserveType::CaseReserve);
        assert_eq!(reserve.amount.amount(), dec!(10000));
        assert!(reserve.reason.is_none());
        assert!(reserve.created_by.is_none());
    }

    #[test]
    fn test_all_reserve_types() {
        let types = vec![
            ReserveType::CaseReserve,
            ReserveType::Ibnr,
            ReserveType::LegalExpense,
            ReserveType::Expense,
        ];

        for reserve_type in types {
            let json = serde_json::to_string(&reserve_type).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Claim Payment Tests
// ============================================================================

mod claim_payment_tests {
    use super::*;

    #[test]
    fn test_claim_payment_new() {
        let claim_id = ClaimId::new_v7();
        let payee_id = PartyId::new_v7();
        let amount = Money::new(dec!(5000), Currency::USD);

        let payment = ClaimPayment::new(
            claim_id,
            payee_id,
            amount,
            PaymentType::Indemnity,
            PaymentMethod::BankTransfer,
        );

        assert_eq!(payment.claim_id, claim_id);
        assert_eq!(payment.payee_id, payee_id);
        assert_eq!(payment.amount.amount(), dec!(5000));
        assert_eq!(payment.payment_type, PaymentType::Indemnity);
        assert_eq!(payment.payment_method, PaymentMethod::BankTransfer);
        assert!(payment.reference.is_none());
    }

    #[test]
    fn test_all_claim_payment_types() {
        let types = vec![
            PaymentType::Indemnity,
            PaymentType::Expense,
            PaymentType::Partial,
            PaymentType::FinalSettlement,
        ];

        for payment_type in types {
            let json = serde_json::to_string(&payment_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_claim_payment_methods() {
        let methods = vec![
            PaymentMethod::BankTransfer,
            PaymentMethod::Check,
            PaymentMethod::DirectDeposit,
            PaymentMethod::Wire,
        ];

        for method in methods {
            let json = serde_json::to_string(&method).unwrap();
            assert!(!json.is_empty());
        }
    }
}
