#![deny(clippy::unwrap_used)]
#![warn(clippy::unchecked_time_subtraction)]

use crate::gitql_schema::{tables_fields_names, tables_fields_types};
use gitql_cli::{arguments::Arguments, diagnostic_reporter, printer::OutputFormatKind};
use gitql_core::{environment::Environment, object::GitQLObject, schema::Schema};
use gitql_data_provider::GitDataProvider;
use gitql_engine::{data_provider::DataProvider, engine, engine::EvaluationResult::SelectedGroups};
use gitql_parser::diagnostic::Diagnostic;
use gitql_parser::{parser, tokenizer};
use gitql_std::aggregation::{aggregation_function_signatures, aggregation_functions};
use nu_plugin::{
    serve_plugin, EngineInterface, EvaluatedCall, MsgPackSerializer, Plugin, PluginCommand,
    SimplePluginCommand,
};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, SyntaxShape, Value};
use std::path::Path;

mod gitql_data_provider;
mod gitql_functions;
mod gitql_schema;
mod nushell_render;

pub struct GitqlPlugin;

impl Plugin for GitqlPlugin {
    fn version(&self) -> String {
        // This automatically uses the version of your package from Cargo.toml as the plugin version
        // sent to Nushell
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            // Commands should be added here
            Box::new(Gitql),
        ]
    }
}

pub struct Gitql;

impl SimplePluginCommand for Gitql {
    type Plugin = GitqlPlugin;

    fn name(&self) -> &str {
        "query git"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .required("query", SyntaxShape::String, "query string")
            .named(
                "repo",
                SyntaxShape::String,
                "Repository path to query",
                Some('r'),
            )
            .named(
                "repos",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Repository paths to query",
                Some('R'),
            )
            .named(
                "output",
                SyntaxShape::String,
                "Output format: table, json, csv, yaml",
                Some('o'),
            )
            .named(
                "page-size",
                SyntaxShape::Int,
                "Pagination page size",
                Some('s'),
            )
            .switch(
                "pagination",
                "Limit output to a single page of results",
                Some('p'),
            )
            .switch("analysis", "Show query analysis timings", Some('a'))
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Use query git to query git repositories"
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "query git 'show tables'",
                description: "Show the tables available to be queried",
                result: None,
            },
            Example {
                example: "query git 'select * from refs limit 10'",
                description: "Show the first 10 refs",
                result: None,
            },
            Example {
                example: "query git 'describe commits' --output yaml",
                description: "Show the commits schema as YAML (JSON, YAML, CSV)",
                result: None,
            },
            Example {
                example: "query git 'select title, datetime from commits' --repo . --output csv",
                description: "Query commits from the current repo and return CSV (JSON, YAML, CSV)",
                result: None,
            },
            Example {
                example: "query git 'show tables' --repos [.]",
                description: "Query multiple repositories using a Nushell list",
                result: None,
            },
            Example {
                example: "query git 'select * from refs' --pagination --page-size 20",
                description: "Limit output to the first 20 rows of results",
                result: None,
            },
            Example {
                example: "query git 'select count(*) from commits' --analysis",
                description: "Run a query and print analysis timing information",
                result: None,
            },
            Example {
                example: r#"query git 'SELECT title, datetime FROM commits WHERE commit_conventional(title) = "feat"'"#,
                description: "Show title and datetime of commits with conventional title 'feat'",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _plugin: &GitqlPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let curdir = engine.get_current_dir()?;
        let query_string: String = call.req(0)?;

        let repo_flag: Option<String> = call
            .get_flag("repo")
            .map_err(|err| LabeledError::new(err.to_string()))?;
        let repos_flag: Option<Value> = call
            .get_flag("repos")
            .map_err(|err| LabeledError::new(err.to_string()))?;
        let output_flag: Option<String> = call
            .get_flag("output")
            .map_err(|err| LabeledError::new(err.to_string()))?;
        let page_size_flag: Option<i64> = call
            .get_flag("page-size")
            .map_err(|err| LabeledError::new(err.to_string()))?;
        let pagination = call
            .has_flag("pagination")
            .map_err(|err| LabeledError::new(err.to_string()))?;
        let analysis = call
            .has_flag("analysis")
            .map_err(|err| LabeledError::new(err.to_string()))?;

        let repo_paths = parse_repo_paths(&curdir, repo_flag, repos_flag)?;
        let output_format = resolve_output_format(output_flag);

        let query_arguments = Arguments {
            repos: repo_paths,
            output_format,
            pagination,
            page_size: page_size_flag.unwrap_or(10).max(1) as usize,
            analysis,
            enable_line_editor: false,
        };

        let mut reporter = diagnostic_reporter::DiagnosticReporter::default();
        if let Some(schema_value) =
            render_schema_query(&query_string, &query_arguments.output_format)
        {
            return Ok(schema_value);
        }

        let repos = match validate_git_repositories(&query_arguments.repos) {
            Ok(repos) => repos,
            Err(error) => {
                reporter.report_diagnostic(&query_string, Diagnostic::error(error.as_str()));
                return Err(LabeledError::new("Invalid repositories paths"));
            }
        };
        let schema = Schema {
            tables_fields_names: tables_fields_names().to_owned(),
            tables_fields_types: tables_fields_types().to_owned(),
        };

        let std_signatures = gitql_functions::gitql_std_signatures();
        let std_functions = gitql_functions::gitql_std_functions();

        let aggregation_signatures = aggregation_function_signatures();
        let aggregation_functions = aggregation_functions();

        let mut env = Environment::new(schema);
        env.with_standard_functions(&std_signatures, std_functions);
        env.with_aggregation_functions(&aggregation_signatures, aggregation_functions);

        execute_gitql_query(query_string, &query_arguments, &repos, &mut env)

        // Ok(Value::nothing(call.head))
    }
}

fn main() {
    serve_plugin(&GitqlPlugin, MsgPackSerializer);
}

fn execute_gitql_query(
    query: String,
    query_arguments: &Arguments,
    repos: &[gix::Repository],
    env: &mut Environment,
) -> Result<Value, LabeledError> {
    let front_start = std::time::Instant::now();
    let normalized_query = normalize_query(&query);
    let tokens = match tokenizer::Tokenizer::tokenize(&normalized_query) {
        Ok(tokens) => tokens,
        Err(diagnostic) => {
            let diagnostic = *diagnostic;
            return Err(diagnostic_to_labeled_error(&query, diagnostic));
        }
    };
    if tokens.is_empty() {
        return Err(LabeledError::new("No tokens to parse"));
    }

    // eprintln!("3");
    let query_node = match parser::parse_gql(tokens, env) {
        Ok(query_node) => query_node,
        Err(diagnostic) => {
            let diagnostic = *diagnostic;
            return Err(diagnostic_to_labeled_error(&query, diagnostic));
        }
    };
    let front_duration = front_start.elapsed();

    let engine_start = std::time::Instant::now();
    let provider: Box<dyn DataProvider> = Box::new(GitDataProvider::new(repos.to_vec()));
    let engine_results = match engine::evaluate(env, &provider, query_node) {
        Ok(results) => results,
        Err(error) => {
            return Err(diagnostic_to_labeled_error(
                &query,
                Diagnostic::exception(&error),
            ));
        }
    };

    // eprintln!("5");

    // Render the result only if they are selected groups not any other statement
    let engine_result = engine_results.into_iter().last();
    let output: Value = if let Some(SelectedGroups(mut groups)) = engine_result {
        if query_arguments.pagination {
            apply_pagination(&mut groups, query_arguments.page_size);
        }

        match query_arguments.output_format {
            OutputFormatKind::Table => nushell_render::render_objects(&mut groups),
            OutputFormatKind::JSON => {
                if let Some(json) = nushell_render::render_groups_to_json(&mut groups) {
                    Value::test_string(json)
                } else {
                    Value::test_string("No JSON data to show".to_string())
                }
            }
            OutputFormatKind::YAML => {
                if let Some(yaml) = nushell_render::render_groups_to_yaml(&mut groups) {
                    Value::test_string(yaml)
                } else {
                    Value::test_string("No YAML data to show".to_string())
                }
            }
            OutputFormatKind::CSV => {
                if let Some(csv) = nushell_render::render_groups_to_csv(&mut groups) {
                    Value::test_string(csv)
                } else {
                    Value::test_string("No CSV data to show".to_string())
                }
            }
        }
    } else {
        // eprintln!("7");

        Value::test_string("Not a SelectedGroups result".to_string())
    };

    let engine_duration = engine_start.elapsed();

    if query_arguments.analysis {
        eprintln!("\n");
        eprintln!("Analysis:");
        eprintln!("Frontend : {:?}", front_duration);
        eprintln!("Engine   : {:?}", engine_duration);
        eprintln!("Total    : {:?}", (front_duration + engine_duration));
        eprintln!("\n");
    }

    Ok(output)
}

fn apply_pagination(groups: &mut GitQLObject, page_size: usize) {
    if page_size == 0 {
        return;
    }

    for group in &mut groups.groups {
        if group.rows.len() > page_size {
            group.rows.truncate(page_size);
        }
    }
}

fn normalize_query(query: &str) -> String {
    let mut normalized = String::with_capacity(query.len());
    let lower_query = query.to_lowercase();
    let mut index = 0;

    while let Some(start) = lower_query[index..].find("count") {
        let start = index + start;
        normalized.push_str(&query[index..start]);

        let mut pos = start + "count".len();
        while pos < query.len() && query.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if pos < query.len() && query.as_bytes()[pos] == b'(' {
            let mut closing = pos + 1;
            let mut has_star = false;
            let mut valid = true;
            while closing < query.len() {
                let c = query.as_bytes()[closing];
                if c == b')' {
                    break;
                }
                if !c.is_ascii_whitespace() {
                    if c == b'*' {
                        has_star = true;
                    } else {
                        valid = false;
                    }
                }
                closing += 1;
            }

            if valid && has_star && closing < query.len() && query.as_bytes()[closing] == b')' {
                normalized.push_str("count(1)");
                index = closing + 1;
                continue;
            }
        }

        normalized.push_str(&query[start..start + "count".len()]);
        index = start + "count".len();
    }

    normalized.push_str(&query[index..]);
    normalized
}

fn render_schema_query(query: &str, output_format: &OutputFormatKind) -> Option<Value> {
    let normalized = query.trim();
    let rows: Option<Vec<Value>> = if normalized.eq_ignore_ascii_case("show tables") {
        Some(
            gitql_schema::tables()
                .into_iter()
                .map(|table| {
                    let mut record = nu_protocol::Record::new();
                    record.insert("table", Value::test_string(table.to_string()));
                    Value::test_record(record)
                })
                .collect(),
        )
    } else {
        let lower = normalized.to_lowercase();
        if let Some(rest) = lower.strip_prefix("describe ") {
            let table = rest.trim();
            gitql_schema::describe_table(table).map(|fields| {
                fields
                    .into_iter()
                    .map(|(name, type_name)| {
                        let mut record = nu_protocol::Record::new();
                        record.insert("field", Value::test_string(name.to_string()));
                        record.insert("type", Value::test_string(type_name));
                        Value::test_record(record)
                    })
                    .collect()
            })
        } else {
            None
        }
    };

    let rows = rows?;
    match output_format {
        OutputFormatKind::Table => Some(Value::test_list(rows)),
        OutputFormatKind::JSON => render_value_list_to_json(&rows).map(Value::test_string),
        OutputFormatKind::CSV => render_value_list_to_csv(&rows).map(Value::test_string),
        OutputFormatKind::YAML => render_value_list_to_yaml(&rows).map(Value::test_string),
    }
}

fn render_value_list_to_yaml(rows: &[Value]) -> Option<String> {
    let mut elements: Vec<serde_yaml::Value> = Vec::with_capacity(rows.len());

    for row in rows {
        let record = row.as_record().ok()?;
        let mut object = serde_yaml::Mapping::new();
        for (name, value) in record.iter() {
            object.insert(
                serde_yaml::Value::String(name.clone()),
                serde_yaml::Value::String(value.clone().coerce_into_string().unwrap_or_default()),
            );
        }
        elements.push(serde_yaml::Value::Mapping(object));
    }

    serde_yaml::to_string(&elements).ok()
}

fn render_value_list_to_json(rows: &[Value]) -> Option<String> {
    let mut elements = Vec::with_capacity(rows.len());

    for row in rows {
        let record = row.as_record().ok()?;
        let mut object = serde_json::Map::new();
        for (name, value) in record.iter() {
            let string_value = value.clone().coerce_into_string().unwrap_or_default();
            object.insert(name.clone(), serde_json::Value::String(string_value));
        }
        elements.push(serde_json::Value::Object(object));
    }

    serde_json::to_string(&serde_json::Value::Array(elements)).ok()
}

fn render_value_list_to_csv(rows: &[Value]) -> Option<String> {
    let first_row = rows.first()?;
    let first_record = first_row.as_record().ok()?;
    let headers: Vec<String> = first_record.iter().map(|(name, _)| name.clone()).collect();

    let mut writer = csv::Writer::from_writer(vec![]);
    writer.write_record(&headers).ok()?;

    for row in rows {
        let record = row.as_record().ok()?;
        let values: Vec<String> = record
            .iter()
            .map(|(_, value)| value.clone().coerce_into_string().unwrap_or_default())
            .collect();
        writer.write_record(values).ok()?;
    }

    writer
        .into_inner()
        .ok()
        .and_then(|writer_content| String::from_utf8(writer_content).ok())
}

fn diagnostic_to_labeled_error(_query: &str, diagnostic: Diagnostic) -> LabeledError {
    let mut error = LabeledError::new(diagnostic.message().to_string());
    if let Some(location) = diagnostic.location() {
        let span = Span::new(
            location.column_start.saturating_sub(1) as usize,
            location.column_end as usize,
        );
        error = error.with_label(diagnostic.label().to_string(), span);
    }
    if !diagnostic.helps().is_empty() {
        error = error.with_help(diagnostic.helps().join(" "));
    }
    error
}

fn resolve_repo_path(repo: &str, current_dir: &str) -> String {
    let repo_path = Path::new(repo);
    if repo_path.is_absolute() {
        repo.to_string()
    } else {
        Path::new(current_dir)
            .join(repo_path)
            .to_string_lossy()
            .into_owned()
    }
}

/// Normalize repository flag inputs into an absolute set of paths.
fn parse_repo_paths(
    current_dir: &str,
    repo_flag: Option<String>,
    repos_flag: Option<Value>,
) -> Result<Vec<String>, LabeledError> {
    if let Some(repo) = repo_flag {
        return Ok(vec![resolve_repo_path(&repo, current_dir)]);
    }

    if let Some(repos) = repos_flag {
        let repos = repos
            .as_list()
            .map_err(|err| LabeledError::new(err.to_string()))?;
        let repo_paths = repos
            .iter()
            .map(|value| {
                value
                    .clone()
                    .coerce_into_string()
                    .map_err(|err| LabeledError::new(err.to_string()))
                    .map(|repo| resolve_repo_path(&repo, current_dir))
            })
            .collect::<Result<Vec<String>, LabeledError>>()?;

        return Ok(repo_paths);
    }

    Ok(vec![current_dir.to_string()])
}

/// Resolve the output format name into a `OutputFormatKind`.
fn resolve_output_format(output_flag: Option<String>) -> OutputFormatKind {
    match output_flag.as_deref().map(str::to_lowercase).as_deref() {
        Some("json") => OutputFormatKind::JSON,
        Some("csv") => OutputFormatKind::CSV,
        Some("yaml") | Some("yml") => OutputFormatKind::YAML,
        _ => OutputFormatKind::Table,
    }
}

fn validate_git_repositories(repositories: &[String]) -> Result<Vec<gix::Repository>, String> {
    repositories
        .iter()
        .map(|repository| gix::open(repository).map_err(|err| err.to_string()))
        .collect()
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    // This will automatically run the examples specified in your command and compare their actual
    // output against what was specified in the example. You can remove this test if the examples
    // can't be tested this way, but we recommend including it if possible.

    PluginTest::new("query git", GitqlPlugin.into())?.test_command_examples(&Gitql)
}

#[test]
fn test_show_tables_schema_query() {
    let value = render_schema_query("show tables", &OutputFormatKind::Table).unwrap();
    let list = value.as_list().expect("expected list");
    assert!(!list.is_empty());
}

#[test]
fn test_describe_commits_schema_query() {
    let value = render_schema_query("describe commits", &OutputFormatKind::Table).unwrap();
    let list = value.as_list().expect("expected list");
    assert!(list.iter().any(|row| {
        row.as_record()
            .ok()
            .and_then(|record| record.get("field"))
            .and_then(|value| value.as_str().ok())
            .map_or(false, |field| field == "commit_id")
    }));
}

#[test]
fn test_describe_commits_schema_query_includes_repo_name() {
    let value = render_schema_query("describe commits", &OutputFormatKind::Table).unwrap();
    let list = value.as_list().expect("expected list");
    assert!(list.iter().any(|row| {
        row.as_record()
            .ok()
            .and_then(|record| record.get("field"))
            .and_then(|value| value.as_str().ok())
            .map_or(false, |field| field == "repo_name")
    }));
}

#[test]
fn test_describe_commits_yaml_schema_query() {
    let yaml = render_schema_query("describe commits", &OutputFormatKind::YAML)
        .unwrap()
        .coerce_into_string()
        .unwrap();
    assert!(yaml.contains("repo_name"));
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_temp_repo() -> TempDir {
        let temp_dir = TempDir::new().expect("create temp dir");
        let repo_path = temp_dir.path();

        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(repo_path)
                .status()
                .expect("git command failed");
            assert!(status.success(), "git {:?} failed", args);
        };

        run(&["init"]);
        run(&["config", "user.name", "Test User"]);
        run(&["config", "user.email", "test@example.com"]);
        fs::write(repo_path.join("README.md"), "test repo").expect("write file");
        run(&["add", "README.md"]);
        run(&["commit", "-m", "initial commit"]);

        temp_dir
    }

    #[test]
    fn test_git_data_provider_includes_repo_name() {
        let repo_dir = init_temp_repo();
        let repo_path = repo_dir.path();
        let repo_name = repo_path.file_name().unwrap().to_string_lossy().to_string();

        let repo = gix::open(repo_path).expect("open repo");
        let provider = GitDataProvider::new(vec![repo]);
        let rows = provider
            .provide(
                "refs",
                &[
                    "name".to_string(),
                    "repo".to_string(),
                    "repo_name".to_string(),
                ],
            )
            .expect("provide refs");

        assert!(!rows.is_empty());
        assert!(rows.iter().any(|row| {
            row.values
                .get(2)
                .and_then(|value| value.as_text())
                .map_or(false, |value| value == repo_name)
        }));
    }

    #[test]
    fn test_git_data_provider_commits_return_repo_fields() {
        let repo_dir = init_temp_repo();
        let repo_path = repo_dir.path();
        let repo_name = repo_path.file_name().unwrap().to_string_lossy().to_string();

        let repo = gix::open(repo_path).expect("open repo");
        let provider = GitDataProvider::new(vec![repo]);
        let rows = provider
            .provide(
                "commits",
                &[
                    "commit_id".to_string(),
                    "repo".to_string(),
                    "repo_name".to_string(),
                ],
            )
            .expect("provide commits");

        assert!(!rows.is_empty());
        assert!(rows.iter().any(|row| {
            row.values
                .get(2)
                .and_then(|value| value.as_text())
                .map_or(false, |value| value == repo_name)
        }));
    }

    #[test]
    fn test_select_count_with_analysis_returns_value() {
        let repo_dir = init_temp_repo();
        let repo_path = repo_dir.path().to_string_lossy().to_string();
        let repo = gix::open(&repo_path).expect("open repo");

        let query_arguments = Arguments {
            repos: vec![repo_path],
            output_format: OutputFormatKind::Table,
            pagination: false,
            page_size: 10,
            analysis: true,
            enable_line_editor: false,
        };

        let schema = Schema {
            tables_fields_names: tables_fields_names().to_owned(),
            tables_fields_types: tables_fields_types().to_owned(),
        };

        let std_signatures = gitql_functions::gitql_std_signatures();
        let std_functions = gitql_functions::gitql_std_functions();
        let aggregation_signatures = aggregation_function_signatures();
        let aggregation_functions = aggregation_functions();

        let mut env = Environment::new(schema);
        env.with_standard_functions(&std_signatures, std_functions);
        env.with_aggregation_functions(&aggregation_signatures, aggregation_functions);

        let value = execute_gitql_query(
            "select count(*) from commits".to_string(),
            &query_arguments,
            &[repo],
            &mut env,
        )
        .expect("execute query");

        assert!(value.as_str().is_ok() || value.as_list().is_ok());
    }
}
