//! Rules Engine Integration
//!
//! This module provides integration with the zen-engine for dynamic
//! product rules evaluation using JDM (JSON Decision Model) format.
//!
//! # Overview
//!
//! The rules engine allows business analysts to configure product rules
//! without code changes by defining rules in JSON format that are evaluated
//! at runtime.
//!
//! # Example
//!
//! ```rust,ignore
//! use domain_policy::rules_engine::{RulesEngine, ProductRules};
//!
//! // Load product rules from JSON
//! let rules_json = include_str!("../../products/term_life.json");
//! let engine = RulesEngine::new();
//! let rules = engine.load_product_rules(rules_json)?;
//!
//! // Evaluate rules against an application
//! let context = json!({
//!     "applicant": { "age": 35, "gender": "male" },
//!     "medical": { "bmi": 24.5, "is_smoker": false },
//!     "coverage": { "sum_assured": 500000, "term_years": 20 }
//! });
//!
//! let result = engine.evaluate(&rules, context).await?;
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during rules evaluation
#[derive(Debug, Error)]
pub enum RulesError {
    /// Failed to parse rules JSON
    #[error("Failed to parse rules: {0}")]
    ParseError(String),

    /// Rules file not found
    #[error("Rules file not found: {0}")]
    FileNotFound(String),

    /// Evaluation error
    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    /// Invalid rule format
    #[error("Invalid rule format: {0}")]
    InvalidFormat(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Product metadata extracted from rules file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductMetadata {
    /// Product code identifier
    pub product_code: String,
    /// Product display name
    pub product_name: String,
    /// Version of the rules
    pub version: String,
    /// Effective date of rules
    pub effective_date: String,
    /// Currency for premiums
    pub currency: String,
    /// Minimum entry age
    pub min_entry_age: Option<u32>,
    /// Maximum entry age
    pub max_entry_age: Option<u32>,
    /// Minimum term in years (for term products)
    pub min_term_years: Option<u32>,
    /// Maximum term in years
    pub max_term_years: Option<u32>,
    /// Minimum sum assured
    pub min_sum_assured: Option<Decimal>,
    /// Maximum sum assured
    pub max_sum_assured: Option<Decimal>,
    /// Base coverages included
    pub coverages: Vec<String>,
    /// Optional riders available
    pub optional_riders: Option<Vec<String>>,
}

/// Loaded product rules ready for evaluation
#[derive(Debug, Clone)]
pub struct ProductRules {
    /// Raw JSON decision model
    pub jdm: Value,
    /// Extracted product metadata
    pub metadata: ProductMetadata,
    /// Decision nodes extracted from JDM
    nodes: HashMap<String, Value>,
}

impl ProductRules {
    /// Creates new product rules from parsed JDM
    ///
    /// # Arguments
    ///
    /// * `jdm` - Parsed JSON Decision Model
    ///
    /// # Returns
    ///
    /// ProductRules instance or error if invalid format
    pub fn from_jdm(jdm: Value) -> Result<Self, RulesError> {
        // Extract metadata
        let metadata = jdm
            .get("metadata")
            .ok_or_else(|| RulesError::MissingField("metadata".to_string()))?;

        let metadata: ProductMetadata = serde_json::from_value(metadata.clone())
            .map_err(|e| RulesError::ParseError(format!("Invalid metadata: {}", e)))?;

        // Extract nodes
        let nodes_array = jdm
            .get("nodes")
            .and_then(|n| n.as_array())
            .ok_or_else(|| RulesError::MissingField("nodes".to_string()))?;

        let mut nodes = HashMap::new();
        for node in nodes_array {
            if let Some(id) = node.get("id").and_then(|i| i.as_str()) {
                nodes.insert(id.to_string(), node.clone());
            }
        }

        Ok(Self {
            jdm,
            metadata,
            nodes,
        })
    }

    /// Gets a node by ID
    pub fn get_node(&self, id: &str) -> Option<&Value> {
        self.nodes.get(id)
    }

    /// Gets all decision table nodes
    pub fn get_decision_tables(&self) -> Vec<&Value> {
        self.nodes
            .values()
            .filter(|n| {
                n.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "decisionTableNode")
                    .unwrap_or(false)
            })
            .collect()
    }
}

/// Result of rules evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Whether the application is eligible
    pub eligible: bool,
    /// Eligibility reason/message
    pub eligibility_reason: Option<String>,
    /// Calculated base rate per thousand
    pub base_rate_per_thousand: Option<Decimal>,
    /// Smoker loading percentage
    pub smoker_loading_percent: Option<Decimal>,
    /// BMI loading percentage
    pub bmi_loading_percent: Option<Decimal>,
    /// Occupation loading percentage
    pub occupation_loading_percent: Option<Decimal>,
    /// Family history loading percentage
    pub family_history_loading_percent: Option<Decimal>,
    /// Total loading percentage
    pub total_loading_percent: Option<Decimal>,
    /// Calculated annual premium
    pub annual_premium: Option<Decimal>,
    /// Calculated monthly premium
    pub monthly_premium: Option<Decimal>,
    /// Risk classification
    pub risk_class: Option<String>,
    /// Action to take (accept, refer, decline)
    pub action: Option<String>,
    /// Whether medical exam is required
    pub medical_exam_required: Option<bool>,
    /// Underwriting type (simplified, full)
    pub underwriting_type: Option<String>,
    /// Additional output values
    pub additional: HashMap<String, Value>,
}

impl Default for EvaluationResult {
    fn default() -> Self {
        Self {
            eligible: false,
            eligibility_reason: None,
            base_rate_per_thousand: None,
            smoker_loading_percent: None,
            bmi_loading_percent: None,
            occupation_loading_percent: None,
            family_history_loading_percent: None,
            total_loading_percent: None,
            annual_premium: None,
            monthly_premium: None,
            risk_class: None,
            action: None,
            medical_exam_required: None,
            underwriting_type: None,
            additional: HashMap::new(),
        }
    }
}

/// Rules engine for evaluating JDM decision models
///
/// The RulesEngine provides methods for loading product rules from JSON
/// and evaluating them against application contexts.
pub struct RulesEngine {
    /// Cached product rules by product code
    products: HashMap<String, Arc<ProductRules>>,
}

impl RulesEngine {
    /// Creates a new rules engine
    pub fn new() -> Self {
        Self {
            products: HashMap::new(),
        }
    }

    /// Loads product rules from a JSON string
    ///
    /// # Arguments
    ///
    /// * `json_str` - JSON string containing JDM rules
    ///
    /// # Returns
    ///
    /// Parsed ProductRules or error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let engine = RulesEngine::new();
    /// let rules = engine.load_rules_from_str(json_str)?;
    /// ```
    pub fn load_rules_from_str(&self, json_str: &str) -> Result<ProductRules, RulesError> {
        let jdm: Value = serde_json::from_str(json_str)
            .map_err(|e| RulesError::ParseError(e.to_string()))?;

        ProductRules::from_jdm(jdm)
    }

    /// Loads product rules from a file path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JDM JSON file
    ///
    /// # Returns
    ///
    /// Parsed ProductRules or error
    pub fn load_rules_from_file(&self, path: &Path) -> Result<ProductRules, RulesError> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| RulesError::FileNotFound(path.display().to_string()))?;

        self.load_rules_from_str(&content)
    }

    /// Registers product rules for caching
    ///
    /// # Arguments
    ///
    /// * `rules` - Product rules to register
    pub fn register_product(&mut self, rules: ProductRules) {
        let code = rules.metadata.product_code.clone();
        self.products.insert(code, Arc::new(rules));
    }

    /// Gets cached product rules by product code
    ///
    /// # Arguments
    ///
    /// * `product_code` - The product code to look up
    ///
    /// # Returns
    ///
    /// Reference to cached ProductRules if found
    pub fn get_product(&self, product_code: &str) -> Option<Arc<ProductRules>> {
        self.products.get(product_code).cloned()
    }

    /// Evaluates product rules against an application context
    ///
    /// This is a simplified evaluator that processes decision tables
    /// and expression nodes to produce an evaluation result.
    ///
    /// # Arguments
    ///
    /// * `rules` - The product rules to evaluate
    /// * `context` - Application context as JSON
    ///
    /// # Returns
    ///
    /// EvaluationResult containing all computed outputs
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let context = json!({
    ///     "applicant": { "age": 35, "gender": "male" },
    ///     "medical": { "bmi": 24.5, "is_smoker": false }
    /// });
    ///
    /// let result = engine.evaluate(&rules, context)?;
    /// ```
    pub fn evaluate(
        &self,
        rules: &ProductRules,
        context: Value,
    ) -> Result<EvaluationResult, RulesError> {
        let mut result = EvaluationResult::default();
        let mut computed_values: HashMap<String, Value> = HashMap::new();

        // Flatten context for easier access
        if let Some(obj) = context.as_object() {
            for (key, value) in obj {
                computed_values.insert(key.clone(), value.clone());
            }
        }

        // Process decision tables in order
        for table in rules.get_decision_tables() {
            if let Some(content) = table.get("content") {
                self.evaluate_decision_table(content, &context, &mut computed_values)?;
            }
        }

        // Build result from computed values
        result.eligible = computed_values
            .get("eligible")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        result.eligibility_reason = computed_values
            .get("eligibility_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_matches('"').to_string());

        result.base_rate_per_thousand = computed_values
            .get("base_rate_per_thousand")
            .and_then(|v| v.as_f64())
            .map(|f| Decimal::try_from(f).unwrap_or_default());

        result.smoker_loading_percent = computed_values
            .get("smoker_loading_percent")
            .and_then(|v| v.as_f64())
            .map(|f| Decimal::try_from(f).unwrap_or_default());

        result.bmi_loading_percent = computed_values
            .get("bmi_loading_percent")
            .and_then(|v| v.as_f64())
            .map(|f| Decimal::try_from(f).unwrap_or_default());

        result.occupation_loading_percent = computed_values
            .get("occupation_loading_percent")
            .and_then(|v| v.as_f64())
            .map(|f| Decimal::try_from(f).unwrap_or_default());

        result.family_history_loading_percent = computed_values
            .get("family_history_loading_percent")
            .and_then(|v| v.as_f64())
            .map(|f| Decimal::try_from(f).unwrap_or_default());

        result.action = computed_values
            .get("bmi_action")
            .or_else(|| computed_values.get("family_history_action"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim_matches('"').to_string());

        result.medical_exam_required = computed_values
            .get("medical_exam_required")
            .and_then(|v| v.as_bool());

        result.underwriting_type = computed_values
            .get("underwriting_type")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_matches('"').to_string());

        // Calculate totals
        let smoker = result.smoker_loading_percent.unwrap_or_default();
        let bmi = result.bmi_loading_percent.unwrap_or_default();
        let occupation = result.occupation_loading_percent.unwrap_or_default();
        let family = result.family_history_loading_percent.unwrap_or_default();

        result.total_loading_percent = Some(smoker + bmi + occupation + family);

        // Store additional computed values
        for (key, value) in computed_values {
            if !matches!(
                key.as_str(),
                "eligible"
                    | "eligibility_reason"
                    | "base_rate_per_thousand"
                    | "smoker_loading_percent"
                    | "bmi_loading_percent"
                    | "occupation_loading_percent"
                    | "family_history_loading_percent"
            ) {
                result.additional.insert(key, value);
            }
        }

        Ok(result)
    }

    /// Evaluates a single decision table
    fn evaluate_decision_table(
        &self,
        table: &Value,
        context: &Value,
        computed: &mut HashMap<String, Value>,
    ) -> Result<(), RulesError> {
        let rules = table
            .get("rules")
            .and_then(|r| r.as_array())
            .ok_or_else(|| RulesError::InvalidFormat("Missing rules array".to_string()))?;

        let inputs = table
            .get("inputs")
            .and_then(|i| i.as_array())
            .ok_or_else(|| RulesError::InvalidFormat("Missing inputs array".to_string()))?;

        let outputs = table
            .get("outputs")
            .and_then(|o| o.as_array())
            .ok_or_else(|| RulesError::InvalidFormat("Missing outputs array".to_string()))?;

        // Find first matching rule
        for rule in rules {
            if self.rule_matches(rule, inputs, context, computed)? {
                // Apply outputs
                if let Some(rule_outputs) = rule.get("outputs").and_then(|o| o.as_array()) {
                    for (output_def, output_val) in outputs.iter().zip(rule_outputs.iter()) {
                        if let (Some(field), Some(value)) = (
                            output_def.get("field").and_then(|f| f.as_str()),
                            output_val.get("value"),
                        ) {
                            // Parse the value
                            let parsed_value = self.parse_output_value(value);
                            computed.insert(field.to_string(), parsed_value);
                        }
                    }
                }
                break; // First hit policy
            }
        }

        Ok(())
    }

    /// Checks if a rule matches the given context
    fn rule_matches(
        &self,
        rule: &Value,
        inputs: &[Value],
        context: &Value,
        computed: &HashMap<String, Value>,
    ) -> Result<bool, RulesError> {
        let rule_inputs = rule
            .get("inputs")
            .and_then(|i| i.as_array())
            .ok_or_else(|| RulesError::InvalidFormat("Rule missing inputs".to_string()))?;

        for (input_def, rule_input) in inputs.iter().zip(rule_inputs.iter()) {
            let field = input_def
                .get("field")
                .and_then(|f| f.as_str())
                .unwrap_or("");

            let condition = rule_input
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("-");

            // Skip wildcard conditions
            if condition == "-" {
                continue;
            }

            // Get the actual value from context
            let actual_value = self.get_field_value(field, context, computed);

            if !self.matches_condition(&actual_value, condition) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Gets a field value from context using dot notation
    fn get_field_value(
        &self,
        field: &str,
        context: &Value,
        computed: &HashMap<String, Value>,
    ) -> Option<Value> {
        // First check computed values
        if let Some(val) = computed.get(field) {
            return Some(val.clone());
        }

        // Then traverse context
        let parts: Vec<&str> = field.split('.').collect();
        let mut current = context;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current.clone())
    }

    /// Checks if a value matches a condition string
    fn matches_condition(&self, value: &Option<Value>, condition: &str) -> bool {
        let val = match value {
            Some(v) => v,
            None => return false,
        };

        // Handle comparison operators
        if condition.starts_with("< ") || condition.starts_with("<") {
            let threshold = condition.trim_start_matches("< ").trim_start_matches('<');
            return self.compare_less_than(val, threshold);
        }

        if condition.starts_with("> ") || condition.starts_with(">") {
            let threshold = condition.trim_start_matches("> ").trim_start_matches('>');
            return self.compare_greater_than(val, threshold);
        }

        if condition.starts_with(">= ") || condition.starts_with(">=") {
            let threshold = condition.trim_start_matches(">= ").trim_start_matches(">=");
            return self.compare_greater_than_eq(val, threshold);
        }

        if condition.starts_with("<= ") || condition.starts_with("<=") {
            let threshold = condition.trim_start_matches("<= ").trim_start_matches("<=");
            return self.compare_less_than_eq(val, threshold);
        }

        // Handle range conditions [a..b)
        if condition.starts_with('[') || condition.starts_with('(') {
            return self.matches_range(val, condition);
        }

        // Handle equality
        if let Some(s) = val.as_str() {
            return s == condition.trim_matches('"');
        }
        if let Some(b) = val.as_bool() {
            return b.to_string() == condition;
        }
        if let Some(n) = val.as_f64() {
            if let Ok(cond_n) = condition.parse::<f64>() {
                return (n - cond_n).abs() < 0.0001;
            }
        }
        if let Some(n) = val.as_i64() {
            if let Ok(cond_n) = condition.parse::<i64>() {
                return n == cond_n;
            }
        }

        false
    }

    /// Compares if value is less than threshold
    fn compare_less_than(&self, val: &Value, threshold: &str) -> bool {
        if let Some(n) = val.as_f64() {
            if let Ok(t) = threshold.trim().parse::<f64>() {
                return n < t;
            }
        }
        if let Some(n) = val.as_i64() {
            if let Ok(t) = threshold.trim().parse::<i64>() {
                return n < t;
            }
        }
        false
    }

    /// Compares if value is greater than threshold
    fn compare_greater_than(&self, val: &Value, threshold: &str) -> bool {
        if let Some(n) = val.as_f64() {
            if let Ok(t) = threshold.trim().parse::<f64>() {
                return n > t;
            }
        }
        if let Some(n) = val.as_i64() {
            if let Ok(t) = threshold.trim().parse::<i64>() {
                return n > t;
            }
        }
        false
    }

    /// Compares if value is greater than or equal to threshold
    fn compare_greater_than_eq(&self, val: &Value, threshold: &str) -> bool {
        if let Some(n) = val.as_f64() {
            if let Ok(t) = threshold.trim().parse::<f64>() {
                return n >= t;
            }
        }
        if let Some(n) = val.as_i64() {
            if let Ok(t) = threshold.trim().parse::<i64>() {
                return n >= t;
            }
        }
        false
    }

    /// Compares if value is less than or equal to threshold
    fn compare_less_than_eq(&self, val: &Value, threshold: &str) -> bool {
        if let Some(n) = val.as_f64() {
            if let Ok(t) = threshold.trim().parse::<f64>() {
                return n <= t;
            }
        }
        if let Some(n) = val.as_i64() {
            if let Ok(t) = threshold.trim().parse::<i64>() {
                return n <= t;
            }
        }
        false
    }

    /// Checks if value falls within a range like [18..25) or (0..100]
    fn matches_range(&self, val: &Value, range: &str) -> bool {
        let n = match val.as_f64() {
            Some(n) => n,
            None => match val.as_i64() {
                Some(i) => i as f64,
                None => return false,
            },
        };

        // Parse range like [18..25) or (0..100]
        let left_inclusive = range.starts_with('[');
        let right_inclusive = range.ends_with(']');

        let inner = range
            .trim_start_matches(['[', '('])
            .trim_end_matches([')', ']']);

        let parts: Vec<&str> = inner.split("..").collect();
        if parts.len() != 2 {
            return false;
        }

        let low: f64 = match parts[0].trim().parse() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let high: f64 = match parts[1].trim().parse() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let low_ok = if left_inclusive { n >= low } else { n > low };
        let high_ok = if right_inclusive {
            n <= high
        } else {
            n < high
        };

        low_ok && high_ok
    }

    /// Parses an output value from the rule
    fn parse_output_value(&self, value: &Value) -> Value {
        if let Some(s) = value.as_str() {
            // Try to parse as number
            if let Ok(n) = s.parse::<f64>() {
                return Value::from(n);
            }
            // Try to parse as bool
            if s == "true" {
                return Value::Bool(true);
            }
            if s == "false" {
                return Value::Bool(false);
            }
            // Return as string (may be quoted)
            return Value::String(s.to_string());
        }
        value.clone()
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_rules_json() -> &'static str {
        r#"{
            "name": "Test Product",
            "nodes": [
                {
                    "id": "age_check",
                    "type": "decisionTableNode",
                    "content": {
                        "hitPolicy": "first",
                        "inputs": [
                            { "id": "age", "field": "applicant.age" }
                        ],
                        "outputs": [
                            { "id": "eligible", "field": "eligible" },
                            { "id": "reason", "field": "eligibility_reason" }
                        ],
                        "rules": [
                            {
                                "inputs": [{ "id": "age", "value": "< 18" }],
                                "outputs": [{ "id": "eligible", "value": "false" }, { "id": "reason", "value": "\"Too young\"" }]
                            },
                            {
                                "inputs": [{ "id": "age", "value": "[18..65]" }],
                                "outputs": [{ "id": "eligible", "value": "true" }, { "id": "reason", "value": "\"Eligible\"" }]
                            },
                            {
                                "inputs": [{ "id": "age", "value": "> 65" }],
                                "outputs": [{ "id": "eligible", "value": "false" }, { "id": "reason", "value": "\"Too old\"" }]
                            }
                        ]
                    }
                }
            ],
            "edges": [],
            "metadata": {
                "product_code": "TEST_01",
                "product_name": "Test Product",
                "version": "1.0.0",
                "effective_date": "2024-01-01",
                "currency": "USD",
                "coverages": ["death_benefit"]
            }
        }"#
    }

    #[test]
    fn test_load_rules() {
        let engine = RulesEngine::new();
        let rules = engine.load_rules_from_str(sample_rules_json()).unwrap();

        assert_eq!(rules.metadata.product_code, "TEST_01");
        assert_eq!(rules.metadata.product_name, "Test Product");
    }

    #[test]
    fn test_evaluate_eligible() {
        let engine = RulesEngine::new();
        let rules = engine.load_rules_from_str(sample_rules_json()).unwrap();

        let context = json!({
            "applicant": { "age": 35 }
        });

        let result = engine.evaluate(&rules, context).unwrap();
        assert!(result.eligible);
        assert_eq!(result.eligibility_reason, Some("Eligible".to_string()));
    }

    #[test]
    fn test_evaluate_too_young() {
        let engine = RulesEngine::new();
        let rules = engine.load_rules_from_str(sample_rules_json()).unwrap();

        let context = json!({
            "applicant": { "age": 16 }
        });

        let result = engine.evaluate(&rules, context).unwrap();
        assert!(!result.eligible);
        assert_eq!(result.eligibility_reason, Some("Too young".to_string()));
    }

    #[test]
    fn test_evaluate_too_old() {
        let engine = RulesEngine::new();
        let rules = engine.load_rules_from_str(sample_rules_json()).unwrap();

        let context = json!({
            "applicant": { "age": 70 }
        });

        let result = engine.evaluate(&rules, context).unwrap();
        assert!(!result.eligible);
        assert_eq!(result.eligibility_reason, Some("Too old".to_string()));
    }

    #[test]
    fn test_range_matching() {
        let engine = RulesEngine::new();

        // Test inclusive lower bound
        assert!(engine.matches_range(&Value::from(18), "[18..25)"));
        assert!(!engine.matches_range(&Value::from(17), "[18..25)"));

        // Test exclusive upper bound
        assert!(engine.matches_range(&Value::from(24), "[18..25)"));
        assert!(!engine.matches_range(&Value::from(25), "[18..25)"));

        // Test inclusive upper bound
        assert!(engine.matches_range(&Value::from(25), "[18..25]"));
    }
}
