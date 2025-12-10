# Claude Code Instructions for Open Insurance Core

## Code Documentation Standards

When writing or modifying code in this repository:

1. **Always add comprehensive comments** explaining the purpose and logic of non-trivial code blocks
2. **Use auto-docgen compatible documentation comments** for all public items:
   - For Rust: Use `///` for items and `//!` for module-level documentation
   - Include `# Examples` sections where appropriate
   - Document all parameters, return values, and potential errors
3. **Document all public functions, structs, enums, and traits** with:
   - A brief summary line
   - Detailed description if needed
   - Parameter descriptions using `# Arguments`
   - Return value descriptions using `# Returns`
   - Error conditions using `# Errors`
   - Code examples using `# Examples`

## Rust Documentation Example

```rust
/// Calculates the premium for a term life insurance policy.
///
/// This function uses actuarial mortality tables to compute the net premium
/// based on the insured's age, policy term, and sum assured.
///
/// # Arguments
///
/// * `age` - The current age of the insured in years
/// * `term` - The policy term in years
/// * `sum_assured` - The death benefit amount
///
/// # Returns
///
/// The calculated annual premium as a `Decimal` value
///
/// # Errors
///
/// Returns `PremiumError::InvalidAge` if age is outside valid range (0-120)
/// Returns `PremiumError::InvalidTerm` if term exceeds maximum allowed
///
/// # Examples
///
/// ```rust
/// let premium = calculate_term_premium(35, 20, dec!(500000))?;
/// assert!(premium > dec!(0));
/// ```
pub fn calculate_term_premium(age: u32, term: u32, sum_assured: Decimal) -> Result<Decimal, PremiumError> {
    // Implementation
}
```

## Architecture Notes

This is a modular monolith insurance core system built in Rust following hexagonal architecture:

- `core_kernel`: Core types (Money, Time, Identifiers)
- `domain_*`: Domain logic crates (policy, claims, billing, fund, party)
- `infra_db`: Database infrastructure with SQLx
- `interface_api`: HTTP API with Axum
