//! Underwriting domain logic
//!
//! This module handles underwriting rules, risk assessment, and
//! decision-making for policy applications.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Utc, Datelike};

use crate::coverage::Coverage;
use crate::error::PolicyError;

/// Risk classification levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskClass {
    /// Preferred Plus - best rates
    PreferredPlus,
    /// Preferred - better than standard
    Preferred,
    /// Standard - normal rates
    Standard,
    /// Substandard - higher rates
    Substandard,
    /// Table rated - rated by table
    TableRated(u8),
    /// Declined - uninsurable
    Declined,
}

impl RiskClass {
    /// Returns the rate multiplier for this risk class
    ///
    /// A multiplier of 1.0 is standard rates, >1.0 is higher rates
    pub fn rate_multiplier(&self) -> Decimal {
        match self {
            RiskClass::PreferredPlus => dec!(0.75),
            RiskClass::Preferred => dec!(0.85),
            RiskClass::Standard => dec!(1.0),
            RiskClass::Substandard => dec!(1.25),
            RiskClass::TableRated(table) => dec!(1.0) + Decimal::from(*table) * dec!(0.25),
            RiskClass::Declined => dec!(0), // N/A
        }
    }
}

/// Underwriting decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderwritingDecision {
    /// Assigned risk class
    pub risk_class: RiskClass,
    /// Reasons for the decision
    pub reasons: Vec<String>,
    /// Exclusions to be applied
    pub exclusions: Vec<UnderwritingExclusion>,
    /// Loading percentage (if any)
    pub loading_percent: Option<Decimal>,
    /// Modified coverages (if any)
    pub coverage_modifications: Vec<CoverageModification>,
    /// Required documents
    pub required_documents: Vec<String>,
    /// Underwriter notes
    pub notes: Option<String>,
}

/// An exclusion applied during underwriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderwritingExclusion {
    /// Exclusion code
    pub code: String,
    /// Description
    pub description: String,
    /// Duration (None = permanent)
    pub duration_years: Option<u32>,
}

/// Coverage modification from underwriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageModification {
    /// Original coverage ID
    pub coverage_id: uuid::Uuid,
    /// Whether coverage is accepted
    pub accepted: bool,
    /// Modified sum assured (if different)
    pub modified_sum_assured: Option<Decimal>,
    /// Additional loading
    pub loading_percent: Option<Decimal>,
}

/// Application data for underwriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderwritingApplication {
    /// Applicant information
    pub applicant: ApplicantInfo,
    /// Medical history
    pub medical_history: MedicalHistory,
    /// Lifestyle information
    pub lifestyle: LifestyleInfo,
    /// Financial information
    pub financial: FinancialInfo,
    /// Requested coverages
    pub coverages: Vec<Coverage>,
}

/// Applicant personal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicantInfo {
    /// Date of birth
    pub date_of_birth: NaiveDate,
    /// Gender
    pub gender: Gender,
    /// Occupation
    pub occupation: String,
    /// Occupation class (1-4, higher = more hazardous)
    pub occupation_class: u8,
    /// Country of residence
    pub country: String,
}

impl ApplicantInfo {
    /// Calculates age in years
    pub fn age(&self) -> u32 {
        let today = Utc::now().date_naive();
        let mut age = today.year() - self.date_of_birth.year();
        if today.ordinal() < self.date_of_birth.ordinal() {
            age -= 1;
        }
        age as u32
    }
}

/// Gender options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Other,
}

/// Medical history for underwriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalHistory {
    /// Height in cm
    pub height_cm: u32,
    /// Weight in kg
    pub weight_kg: f32,
    /// Current smoker
    pub is_smoker: bool,
    /// Former smoker (quit within last 12 months)
    pub is_former_smoker: bool,
    /// Pre-existing conditions
    pub conditions: Vec<MedicalCondition>,
    /// Family medical history
    pub family_history: Vec<FamilyMedicalHistory>,
}

impl MedicalHistory {
    /// Calculates BMI
    pub fn bmi(&self) -> f32 {
        let height_m = self.height_cm as f32 / 100.0;
        self.weight_kg / (height_m * height_m)
    }

    /// Checks if BMI is in healthy range (18.5 - 24.9)
    pub fn is_healthy_bmi(&self) -> bool {
        let bmi = self.bmi();
        bmi >= 18.5 && bmi <= 24.9
    }
}

/// Medical condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalCondition {
    /// ICD-10 code
    pub code: String,
    /// Condition name
    pub name: String,
    /// Date diagnosed
    pub diagnosed_date: Option<NaiveDate>,
    /// Current status
    pub status: ConditionStatus,
    /// Treatment details
    pub treatment: Option<String>,
}

/// Status of a medical condition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionStatus {
    /// Active/current condition
    Active,
    /// Under control with medication
    Controlled,
    /// In remission
    Remission,
    /// Cured/resolved
    Resolved,
}

/// Family medical history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyMedicalHistory {
    /// Relationship (parent, sibling)
    pub relationship: String,
    /// Condition
    pub condition: String,
    /// Age at diagnosis
    pub age_at_diagnosis: Option<u32>,
}

/// Lifestyle information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifestyleInfo {
    /// Participates in hazardous sports
    pub hazardous_sports: Vec<String>,
    /// Aviation activities (pilot, frequent flyer)
    pub aviation: Option<AviationInfo>,
    /// Alcohol consumption level
    pub alcohol_consumption: AlcoholLevel,
    /// Travel to high-risk countries
    pub travel_risk_countries: Vec<String>,
}

/// Aviation activity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AviationInfo {
    /// Type (private pilot, commercial, passenger)
    pub activity_type: String,
    /// Annual flight hours
    pub hours_per_year: u32,
}

/// Alcohol consumption levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlcoholLevel {
    None,
    Light,
    Moderate,
    Heavy,
}

/// Financial information for underwriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialInfo {
    /// Annual income
    pub annual_income: Decimal,
    /// Net worth
    pub net_worth: Decimal,
    /// Existing life insurance coverage
    pub existing_coverage: Decimal,
    /// Purpose of insurance
    pub purpose: InsurancePurpose,
}

/// Purpose of insurance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsurancePurpose {
    FamilyProtection,
    MortgageProtection,
    BusinessProtection,
    KeyPerson,
    Estate,
    Investment,
    Other(String),
}

/// Rule result from underwriting evaluation
#[derive(Debug, Clone)]
pub struct RuleResult {
    pub rule_name: String,
    pub passed: bool,
    pub message: String,
    pub impact: RuleImpact,
}

/// Impact of a rule on underwriting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleImpact {
    /// No impact - informational only
    None,
    /// Adds loading to premium
    Loading(Decimal),
    /// Adds exclusion
    Exclusion,
    /// Requires decline
    Decline,
    /// Requires referral
    Referral,
}

/// Evaluates basic underwriting rules
///
/// # Arguments
///
/// * `application` - The underwriting application to evaluate
///
/// # Returns
///
/// A list of rule evaluation results
///
/// # Example
///
/// ```rust,ignore
/// let results = evaluate_basic_rules(&application);
/// let should_decline = results.iter().any(|r| r.impact == RuleImpact::Decline);
/// ```
pub fn evaluate_basic_rules(application: &UnderwritingApplication) -> Vec<RuleResult> {
    let mut results = Vec::new();

    // Age rules
    let age = application.applicant.age();
    if age < 18 {
        results.push(RuleResult {
            rule_name: "minimum_age".to_string(),
            passed: false,
            message: "Applicant must be at least 18 years old".to_string(),
            impact: RuleImpact::Decline,
        });
    } else if age > 65 {
        results.push(RuleResult {
            rule_name: "maximum_age".to_string(),
            passed: false,
            message: "Applicant exceeds maximum entry age".to_string(),
            impact: RuleImpact::Referral,
        });
    } else {
        results.push(RuleResult {
            rule_name: "age_check".to_string(),
            passed: true,
            message: format!("Age {} is within acceptable range", age),
            impact: RuleImpact::None,
        });
    }

    // BMI rules
    let bmi = application.medical_history.bmi();
    if bmi < 16.0 || bmi > 40.0 {
        results.push(RuleResult {
            rule_name: "bmi_extreme".to_string(),
            passed: false,
            message: format!("BMI {} is outside acceptable range", bmi),
            impact: RuleImpact::Referral,
        });
    } else if bmi < 18.5 || bmi > 30.0 {
        let loading = if bmi < 18.5 {
            dec!(25)
        } else {
            Decimal::from((bmi - 25.0) as i32 * 5)
        };
        results.push(RuleResult {
            rule_name: "bmi_loading".to_string(),
            passed: true,
            message: format!("BMI {} requires loading", bmi),
            impact: RuleImpact::Loading(loading),
        });
    }

    // Smoking rules
    if application.medical_history.is_smoker {
        results.push(RuleResult {
            rule_name: "smoker_status".to_string(),
            passed: true,
            message: "Smoker rates apply".to_string(),
            impact: RuleImpact::Loading(dec!(50)),
        });
    } else if application.medical_history.is_former_smoker {
        results.push(RuleResult {
            rule_name: "former_smoker".to_string(),
            passed: true,
            message: "Former smoker loading applies".to_string(),
            impact: RuleImpact::Loading(dec!(25)),
        });
    }

    // Occupation class rules
    if application.applicant.occupation_class >= 4 {
        results.push(RuleResult {
            rule_name: "occupation_hazard".to_string(),
            passed: true,
            message: "High-risk occupation loading applies".to_string(),
            impact: RuleImpact::Loading(dec!(50)),
        });
    }

    // Financial justification
    let total_requested: rust_decimal::Decimal = application.coverages
        .iter()
        .map(|c| c.sum_assured.amount())
        .sum();

    let income_multiple = if application.financial.annual_income.is_zero() {
        dec!(0)
    } else {
        total_requested / application.financial.annual_income
    };

    if income_multiple > dec!(20) {
        results.push(RuleResult {
            rule_name: "financial_justification".to_string(),
            passed: false,
            message: format!(
                "Requested coverage ({:.0}x income) exceeds maximum multiple",
                income_multiple
            ),
            impact: RuleImpact::Referral,
        });
    }

    results
}

/// Calculates the overall risk class from rule results
///
/// # Arguments
///
/// * `results` - The rule evaluation results
///
/// # Returns
///
/// The determined risk class
pub fn determine_risk_class(results: &[RuleResult]) -> RiskClass {
    // Check for decline
    if results.iter().any(|r| r.impact == RuleImpact::Decline) {
        return RiskClass::Declined;
    }

    // Sum up loadings
    let total_loading: Decimal = results
        .iter()
        .filter_map(|r| {
            if let RuleImpact::Loading(l) = r.impact {
                Some(l)
            } else {
                None
            }
        })
        .sum();

    // Determine risk class based on loading
    if total_loading.is_zero() {
        RiskClass::PreferredPlus
    } else if total_loading <= dec!(10) {
        RiskClass::Preferred
    } else if total_loading <= dec!(25) {
        RiskClass::Standard
    } else if total_loading <= dec!(75) {
        RiskClass::Substandard
    } else {
        // Calculate table rating
        let table = ((total_loading - dec!(75)) / dec!(25)).to_string()
            .parse::<u8>()
            .unwrap_or(1)
            .min(8);
        RiskClass::TableRated(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_class_multiplier() {
        assert_eq!(RiskClass::Standard.rate_multiplier(), dec!(1.0));
        assert!(RiskClass::PreferredPlus.rate_multiplier() < dec!(1.0));
        assert!(RiskClass::Substandard.rate_multiplier() > dec!(1.0));
    }

    #[test]
    fn test_bmi_calculation() {
        let history = MedicalHistory {
            height_cm: 175,
            weight_kg: 70.0,
            is_smoker: false,
            is_former_smoker: false,
            conditions: vec![],
            family_history: vec![],
        };

        let bmi = history.bmi();
        assert!(bmi > 22.0 && bmi < 23.0); // ~22.86
    }

    #[test]
    fn test_age_calculation() {
        let applicant = ApplicantInfo {
            date_of_birth: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            gender: Gender::Male,
            occupation: "Software Engineer".to_string(),
            occupation_class: 1,
            country: "US".to_string(),
        };

        let age = applicant.age();
        assert!(age >= 34 && age <= 35); // Depending on current date
    }
}
