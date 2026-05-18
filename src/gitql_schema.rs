use gitql_ast::types::{
    boolean::BoolType, date::DateType, integer::IntType, text::TextType, DataType,
};
use std::collections::HashMap;
use std::sync::OnceLock;

pub fn tables_fields_types() -> HashMap<&'static str, Box<dyn DataType>> {
    let mut map: HashMap<&'static str, Box<dyn DataType>> = HashMap::new();
    map.insert("commit_id", Box::new(TextType));
    map.insert("title", Box::new(TextType));
    map.insert("message", Box::new(TextType));
    map.insert("name", Box::new(TextType));
    map.insert("author_name", Box::new(TextType));
    map.insert("author_email", Box::new(TextType));
    map.insert("committer_name", Box::new(TextType));
    map.insert("committer_email", Box::new(TextType));
    map.insert("full_name", Box::new(TextType));
    map.insert("insertions", Box::new(IntType));
    map.insert("deletions", Box::new(IntType));
    map.insert("files_changed", Box::new(IntType));
    map.insert("email", Box::new(TextType));
    map.insert("type", Box::new(TextType));
    map.insert("datetime", Box::new(DateType));
    map.insert("is_head", Box::new(BoolType));
    map.insert("is_remote", Box::new(BoolType));
    map.insert("commit_count", Box::new(IntType));
    map.insert("parents_count", Box::new(IntType));
    map.insert("updated", Box::new(DateType));
    map.insert("repo", Box::new(TextType));
    map.insert("repo_name", Box::new(TextType));
    map
}

pub fn tables() -> Vec<&'static str> {
    let mut tables: Vec<&'static str> = tables_fields_names().keys().copied().collect();
    tables.sort();
    tables
}

pub fn describe_table(table: &str) -> Option<Vec<(&'static str, String)>> {
    tables_fields_names().get(table).map(|fields| {
        fields
            .iter()
            .map(|field| {
                let field_type = tables_fields_types()
                    .get(field)
                    .map(|data_type| data_type.literal())
                    .unwrap_or_else(|| "unknown".to_string());
                (*field, field_type)
            })
            .collect()
    })
}

pub fn tables_fields_names() -> &'static HashMap<&'static str, Vec<&'static str>> {
    static HASHMAP: OnceLock<HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(
            "refs",
            vec!["name", "full_name", "type", "repo", "repo_name"],
        );
        map.insert(
            "commits",
            vec![
                "commit_id",
                "title",
                "message",
                "author_name",
                "author_email",
                "committer_name",
                "committer_email",
                "datetime",
                "parents_count",
                "repo",
                "repo_name",
            ],
        );
        map.insert(
            "branches",
            vec![
                "name",
                "commit_count",
                "is_head",
                "is_remote",
                "updated",
                "repo",
                "repo_name",
            ],
        );
        map.insert(
            "diffs",
            vec![
                "commit_id",
                "name",
                "email",
                "insertions",
                "deletions",
                "files_changed",
                "datetime",
                "repo",
                "repo_name",
            ],
        );
        map.insert("tags", vec!["name", "repo", "repo_name"]);
        map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_definitions_expose_repo_name() {
        for table in ["refs", "commits", "branches", "diffs", "tags"] {
            let fields = tables_fields_names().get(table).expect("table exists");
            assert!(
                fields.contains(&"repo_name"),
                "{table} should expose repo_name"
            );
        }
    }
}
