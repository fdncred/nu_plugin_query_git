use gitql_ast::types::text::TextType;
use gitql_core::signature::{Signature, StandardFunction};
use gitql_core::values::text::TextValue;
use gitql_core::values::Value;
use gitql_std::standard::{standard_function_signatures, standard_functions};
use std::collections::HashMap;
use std::sync::OnceLock;

pub fn gitql_std_functions() -> &'static HashMap<&'static str, StandardFunction> {
    static HASHMAP: OnceLock<HashMap<&'static str, StandardFunction>> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        let mut map = standard_functions().to_owned();
        map.insert("commit_conventional", commit_conventional);
        map.insert("commit_type", commit_type);
        map.insert("commit_scope", commit_scope);
        map.insert("commit_description", commit_description);
        map
    })
}

pub fn gitql_std_signatures() -> HashMap<&'static str, Signature> {
    let mut map = standard_function_signatures().to_owned();
    map.insert(
        "commit_conventional",
        Signature {
            parameters: vec![Box::new(TextType)],
            return_type: Box::new(TextType),
        },
    );
    map.insert(
        "commit_type",
        Signature {
            parameters: vec![Box::new(TextType)],
            return_type: Box::new(TextType),
        },
    );
    map.insert(
        "commit_scope",
        Signature {
            parameters: vec![Box::new(TextType)],
            return_type: Box::new(TextType),
        },
    );
    map.insert(
        "commit_description",
        Signature {
            parameters: vec![Box::new(TextType)],
            return_type: Box::new(TextType),
        },
    );
    map
}

fn commit_conventional(values: &[Box<dyn Value>]) -> Box<dyn Value> {
    let text = values
        .first()
        .and_then(|value| value.as_text())
        .unwrap_or_default();
    let split: Vec<&str> = text.split(':').collect();
    let conventional = if split.len() == 1 { "" } else { split[0] };
    Box::new(TextValue {
        value: conventional.to_string(),
    })
}

fn commit_type(values: &[Box<dyn Value>]) -> Box<dyn Value> {
    let text = values
        .first()
        .and_then(|value| value.as_text())
        .unwrap_or_default();
    let commit_type = text
        .split(':')
        .next()
        .and_then(|part| part.split('(').next())
        .unwrap_or("")
        .trim();
    Box::new(TextValue {
        value: commit_type.to_string(),
    })
}

fn commit_scope(values: &[Box<dyn Value>]) -> Box<dyn Value> {
    let text = values
        .first()
        .and_then(|value| value.as_text())
        .unwrap_or_default();
    let scope = text
        .split(':')
        .next()
        .and_then(|part| part.split_once('('))
        .and_then(|(_, after)| after.split_once(')'))
        .map(|(scope, _)| scope)
        .unwrap_or("")
        .trim()
        .to_string();
    Box::new(TextValue { value: scope })
}

fn commit_description(values: &[Box<dyn Value>]) -> Box<dyn Value> {
    let text = values
        .first()
        .and_then(|value| value.as_text())
        .unwrap_or_default();
    let description = text
        .split_once(':')
        .map(|(_, rest)| rest)
        .unwrap_or("")
        .trim()
        .to_string();
    Box::new(TextValue { value: description })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitql_core::values::text::TextValue;

    fn text_value(input: &str) -> Box<dyn Value> {
        Box::new(TextValue {
            value: input.to_string(),
        })
    }

    #[test]
    fn commit_type_extracts_type_from_conventional_message() {
        let value = commit_type(&[text_value("feat(scope): add feature")]);
        assert_eq!(value.as_text().unwrap(), "feat");
    }

    #[test]
    fn commit_scope_extracts_scope_from_conventional_message() {
        let value = commit_scope(&[text_value("feat(scope): add feature")]);
        assert_eq!(value.as_text().unwrap(), "scope");
    }

    #[test]
    fn commit_description_extracts_message_body() {
        let value = commit_description(&[text_value("feat(scope): add feature")]);
        assert_eq!(value.as_text().unwrap(), "add feature");
    }
}
