//! Tests for core_kernel error types

use core_kernel::error::CoreError;
use core_kernel::money::MoneyError;

#[test]
fn test_core_error_validation() {
    let error = CoreError::validation("Invalid input");

    match error {
        CoreError::Validation(msg) => assert_eq!(msg, "Invalid input"),
        _ => panic!("Expected Validation error"),
    }
}

#[test]
fn test_core_error_invalid_state() {
    let error = CoreError::invalid_state("Cannot transition from A to B");

    match error {
        CoreError::InvalidStateTransition(msg) => assert!(msg.contains("Cannot transition")),
        _ => panic!("Expected InvalidStateTransition error"),
    }
}

#[test]
fn test_core_error_not_found() {
    let error = CoreError::not_found("Policy not found");

    match error {
        CoreError::NotFound(msg) => assert_eq!(msg, "Policy not found"),
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_core_error_from_money_error() {
    let money_error = MoneyError::CurrencyMismatch("USD".to_string(), "EUR".to_string());
    let core_error: CoreError = money_error.into();

    assert!(matches!(core_error, CoreError::Money(_)));
}

#[test]
fn test_core_error_display() {
    let error = CoreError::validation("Test error");
    let display = format!("{}", error);

    assert!(display.contains("Validation error"));
}

#[test]
fn test_core_error_configuration() {
    let error = CoreError::Configuration("Missing config".to_string());

    match error {
        CoreError::Configuration(msg) => assert_eq!(msg, "Missing config"),
        _ => panic!("Expected Configuration error"),
    }
}
