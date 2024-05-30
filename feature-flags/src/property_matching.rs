use std::collections::HashMap;

use crate::flag_definitions::{OperatorType, PropertyFilter};
use regex::Regex;
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum FlagMatchingError {
    ValidationError(String),
    MissingProperty(String),
    InconclusiveOperatorMatch,
    InvalidRegexPattern,
}

pub fn match_property(
    property: &PropertyFilter,
    matching_property_values: &HashMap<String, Value>,
    partial_props: bool,
) -> Result<bool, FlagMatchingError> {
    // only looks for matches where key exists in override_property_values
    // doesn't support operator is_not_set with partial_props

    if partial_props {
        if !matching_property_values.contains_key(&property.key) {
            return Err(FlagMatchingError::MissingProperty(format!(
                "can't match properties without a value. Missing property: {}",
                property.key
            )));
        }
    }

    let key = &property.key;
    let operator = property.operator.clone().unwrap_or(OperatorType::Exact);
    let value = &property.value;
    let match_value = matching_property_values.get(key);

    match operator {
        OperatorType::Exact | OperatorType::IsNot => {
            let compute_exact_match = |value: &Value, override_value: &Value| -> bool {
                if is_truthy_or_falsy_property_value(value) {
                    // Do boolean handling, such that passing in "true" or "True" or "false" or "False" as matching value is equivalent
                    let truthy = is_truthy_property_value(value);
                    return override_value.to_string().to_lowercase()
                        == truthy.to_string().to_lowercase();
                }

                if value.is_array() {
                    // TODO: Check if `to_string()` coerces all types to string correctly.
                    return value
                        .as_array()
                        .expect("expected array value")
                        .iter()
                        .map(|v| v.to_string().to_lowercase())
                        .collect::<Vec<String>>()
                        .contains(&override_value.to_string().to_lowercase());
                }
                return value.to_string().to_lowercase()
                    == override_value.to_string().to_lowercase();
            };

            if let Some(match_value) = match_value {
                if operator == OperatorType::Exact {
                    Ok(compute_exact_match(value, match_value))
                } else {
                    Ok(!compute_exact_match(value, match_value))
                }
            } else {
                return Ok(false);
            }
        }
        OperatorType::IsSet => Ok(matching_property_values.contains_key(key)),
        OperatorType::IsNotSet => {
            if partial_props {
                if matching_property_values.contains_key(key) {
                    Ok(false)
                } else {
                    Err(FlagMatchingError::InconclusiveOperatorMatch)
                }
            } else {
                Ok(!matching_property_values.contains_key(key))
            }
        }
        OperatorType::Icontains | OperatorType::NotIcontains => {
            if let Some(match_value) = match_value {
                let is_contained = match_value
                    .to_string()
                    .to_lowercase()
                    .contains(&value.to_string().to_lowercase());
                if operator == OperatorType::Icontains {
                    Ok(is_contained)
                } else {
                    Ok(!is_contained)
                }
            } else {
                // When value doesn't exist, it's not a match
                Ok(false)
            }
        }
        OperatorType::Regex | OperatorType::NotRegex => {
            if match_value.is_none() {
                return Ok(false);
            }

            let pattern = match Regex::new(&value.to_string()) {
                Ok(pattern) => pattern,
                Err(_) => return Err(FlagMatchingError::InvalidRegexPattern),
            };
            let haystack = match_value.unwrap_or(&Value::Null).to_string();
            let match_ = pattern.find(&haystack);

            if operator == OperatorType::Regex {
                Ok(match_.is_some())
            } else {
                Ok(match_.is_none())
            }
        }
        OperatorType::Gt | OperatorType::Gte | OperatorType::Lt | OperatorType::Lte => {
            if match_value.is_none() {
                return Ok(false);
            }
            // TODO: Move towards only numeric matching of these operators???

            let compare = |lhs: f64, rhs: f64, operator: OperatorType| -> bool {
                match operator {
                    OperatorType::Gt => lhs > rhs,
                    OperatorType::Gte => lhs >= rhs,
                    OperatorType::Lt => lhs < rhs,
                    OperatorType::Lte => lhs <= rhs,
                    _ => false,
                }
            };

            let parsed_value = match match_value.unwrap_or(&Value::Null).as_f64() {
                Some(parsed_value) => parsed_value,
                None => {
                    return Err(FlagMatchingError::ValidationError(
                        "value is not a number".to_string(),
                    ))
                }
            };

            if let Some(override_value) = value.as_f64() {
                Ok(compare(override_value, parsed_value, operator))
            } else {
                return Err(FlagMatchingError::ValidationError(
                    "override value is not a number".to_string(),
                ));
            }
        }
        OperatorType::IsDateExact | OperatorType::IsDateAfter | OperatorType::IsDateBefore => {
            // TODO: Handle date operators
            return Ok(false);
            // let parsed_date = determine_parsed_date_for_property_matching(match_value);

            // if parsed_date.is_none() {
            //     return Ok(false);
            // }

            // if let Some(override_value) = value.as_str() {
            //     let override_date = match parser::parse(override_value) {
            //         Ok(override_date) => override_date,
            //         Err(_) => return Ok(false),
            //     };

            //     match operator {
            //         OperatorType::IsDateBefore => Ok(override_date < parsed_date.unwrap()),
            //         OperatorType::IsDateAfter => Ok(override_date > parsed_date.unwrap()),
            //         _ => Ok(false),
            //     }
            // } else {
            //     Ok(false)
            // }
        }
    }
}

fn is_truthy_or_falsy_property_value(value: &Value) -> bool {
    if value.is_boolean() {
        return true;
    }

    if value.is_string() {
        let parsed_value = value
            .as_str()
            .expect("expected string value")
            .to_lowercase();
        return parsed_value == "true" || parsed_value == "false";
    }

    if value.is_array() {
        return value
            .as_array()
            .expect("expected array value")
            .iter()
            .all(|v| is_truthy_or_falsy_property_value(v));
    }

    false
}

fn is_truthy_property_value(value: &Value) -> bool {
    if value.is_boolean() {
        return value.as_bool().expect("expected boolean value");
    }

    if value.is_string() {
        let parsed_value = value
            .as_str()
            .expect("expected string value")
            .to_lowercase();
        return parsed_value == "true";
    }

    if value.is_array() {
        return value
            .as_array()
            .expect("expected array value")
            .iter()
            .all(|v| is_truthy_property_value(v));
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_match_properties_exact_with_partial_props() {
        let property_a = PropertyFilter {
            key: "key".to_string(),
            value: json!("value"),
            operator: None,
            prop_type: "person".to_string(),
            group_type_index: None,
        };

        assert_eq!(
            match_property(
                &property_a,
                &HashMap::from([("key".to_string(), json!("value"))]),
                true
            )
            .expect("expected match to exist"),
            true
        );

        assert_eq!(
            match_property(
                &property_a,
                &HashMap::from([("key".to_string(), json!("value2"))]),
                true
            )
            .expect("expected match to exist"),
            false
        );
        assert_eq!(
            match_property(
                &property_a,
                &HashMap::from([("key".to_string(), json!(""))]),
                true
            )
            .expect("expected match to exist"),
            false
        );
        assert_eq!(
            match_property(
                &property_a,
                &HashMap::from([("key".to_string(), json!(null))]),
                true
            )
            .expect("expected match to exist"),
            false
        );

        assert_eq!(
            match_property(
                &property_a,
                &HashMap::from([("key2".to_string(), json!("value"))]),
                true
            )
            .is_err(),
            true
        );
        assert_eq!(
            match_property(
                &property_a,
                &HashMap::from([("key2".to_string(), json!("value"))]),
                true
            )
            .err()
            .expect("expected match to exist"),
            FlagMatchingError::MissingProperty(
                "can't match properties without a value. Missing property: key".to_string()
            )
        );
        assert_eq!(
            match_property(&property_a, &HashMap::from([]), true).is_err(),
            true
        );

        let property_b = PropertyFilter {
            key: "key".to_string(),
            value: json!("value"),
            operator: Some(OperatorType::Exact),
            prop_type: "person".to_string(),
            group_type_index: None,
        };

        assert_eq!(
            match_property(
                &property_b,
                &HashMap::from([("key".to_string(), json!("value"))]),
                true
            )
            .expect("expected match to exist"),
            true
        );

        assert_eq!(
            match_property(
                &property_b,
                &HashMap::from([("key".to_string(), json!("value2"))]),
                true
            )
            .expect("expected match to exist"),
            false
        );

        let property_c = PropertyFilter {
            key: "key".to_string(),
            value: json!(["value1", "value2", "value3"]),
            operator: Some(OperatorType::Exact),
            prop_type: "person".to_string(),
            group_type_index: None,
        };

        assert_eq!(
            match_property(
                &property_c,
                &HashMap::from([("key".to_string(), json!("value1"))]),
                true
            )
            .expect("expected match to exist"),
            true
        );
        assert_eq!(
            match_property(
                &property_c,
                &HashMap::from([("key".to_string(), json!("value2"))]),
                true
            )
            .expect("expected match to exist"),
            true
        );
        assert_eq!(
            match_property(
                &property_c,
                &HashMap::from([("key".to_string(), json!("value3"))]),
                true
            )
            .expect("expected match to exist"),
            true
        );

        assert_eq!(
            match_property(
                &property_c,
                &HashMap::from([("key".to_string(), json!("value4"))]),
                true
            )
            .expect("expected match to exist"),
            false
        );

        assert_eq!(
            match_property(
                &property_c,
                &HashMap::from([("key2".to_string(), json!("value"))]),
                true
            )
            .is_err(),
            true
        );
    }
}
