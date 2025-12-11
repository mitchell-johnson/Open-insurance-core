//! Tests for domain_policy events

use chrono::{NaiveDate, Utc};
use rust_decimal_macros::dec;

use core_kernel::{PolicyId, EndorsementId};

use domain_policy::events::{PolicyEvent, UnderwritingDecisionType};

#[test]
fn test_policy_quoted_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyQuoted {
        policy_id,
        quote_expiry: now + chrono::Duration::days(30),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyQuoted");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_issued_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyIssued {
        policy_id,
        effective_date: now.date_naive(),
        underwriter: "underwriter@example.com".to_string(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyIssued");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_lapsed_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyLapsed {
        policy_id,
        reason: "Non-payment".to_string(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyLapsed");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_reinstated_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyReinstated {
        policy_id,
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyReinstated");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_terminated_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyTerminated {
        policy_id,
        reason: "Customer request".to_string(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyTerminated");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_cancelled_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyCancelled {
        policy_id,
        reason: "Fraud".to_string(),
        refund_amount: Some(dec!(500)),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyCancelled");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_expired_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyExpired {
        policy_id,
        expiry_date: now.date_naive(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyExpired");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_renewed_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyRenewed {
        policy_id,
        new_effective_date: now.date_naive(),
        new_expiry_date: (now + chrono::Duration::days(365)).date_naive(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyRenewed");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_endorsement_applied_event() {
    let policy_id = PolicyId::new_v7();
    let endorsement_id = EndorsementId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::EndorsementApplied {
        policy_id,
        endorsement_id,
        endorsement_type: "BeneficiaryChange".to_string(),
        effective_date: now.date_naive(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "EndorsementApplied");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_payment_received_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PaymentReceived {
        policy_id,
        amount: dec!(1000),
        currency: "USD".to_string(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PaymentReceived");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_premium_due_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PremiumDue {
        policy_id,
        amount: dec!(500),
        currency: "USD".to_string(),
        due_date: now.date_naive(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PremiumDue");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_premium_overdue_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PremiumOverdue {
        policy_id,
        amount: dec!(500),
        currency: "USD".to_string(),
        days_overdue: 30,
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PremiumOverdue");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_grace_period_started_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::GracePeriodStarted {
        policy_id,
        grace_end_date: (now + chrono::Duration::days(30)).date_naive(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "GracePeriodStarted");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_submitted_for_underwriting_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::SubmittedForUnderwriting {
        policy_id,
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "SubmittedForUnderwriting");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_underwriting_decision_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::UnderwritingDecision {
        policy_id,
        decision: UnderwritingDecisionType::Approved,
        underwriter: "uw@example.com".to_string(),
        notes: Some("Standard risk".to_string()),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "UnderwritingDecision");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_beneficiary_changed_event() {
    let policy_id = PolicyId::new_v7();
    let endorsement_id = EndorsementId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::BeneficiaryChanged {
        policy_id,
        endorsement_id,
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "BeneficiaryChanged");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_loan_taken_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyLoanTaken {
        policy_id,
        amount: dec!(10000),
        currency: "USD".to_string(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyLoanTaken");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_policy_loan_repaid_event() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PolicyLoanRepaid {
        policy_id,
        amount: dec!(5000),
        currency: "USD".to_string(),
        timestamp: now,
    };

    assert_eq!(event.policy_id(), policy_id);
    assert_eq!(event.event_type(), "PolicyLoanRepaid");
    assert_eq!(event.timestamp(), now);
}

#[test]
fn test_all_underwriting_decision_types() {
    let decisions = vec![
        UnderwritingDecisionType::Approved,
        UnderwritingDecisionType::ApprovedWithConditions,
        UnderwritingDecisionType::ApprovedWithRating,
        UnderwritingDecisionType::Declined,
        UnderwritingDecisionType::Postponed,
        UnderwritingDecisionType::Referred,
    ];

    for decision in decisions {
        let json = serde_json::to_string(&decision).unwrap();
        assert!(!json.is_empty());
    }
}

#[test]
fn test_policy_event_serialization() {
    let policy_id = PolicyId::new_v7();
    let now = Utc::now();

    let event = PolicyEvent::PaymentReceived {
        policy_id,
        amount: dec!(1000),
        currency: "USD".to_string(),
        timestamp: now,
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: PolicyEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.policy_id(), policy_id);
}
