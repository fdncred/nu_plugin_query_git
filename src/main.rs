use gitql_ast::object::flat_gql_groups;
use gitql_ast::object::GQLObject;
use gitql_ast::statement::AggregationFunctionsStatement;
use gitql_ast::statement::SelectStatement;
use gitql_ast::statement::StatementKind;
use gitql_engine::engine;
use gitql_parser::parser;
use gitql_parser::tokenizer;
use nu_plugin::{
    serve_plugin, EngineInterface, EvaluatedCall, MsgPackSerializer, Plugin, PluginCommand,
    SimplePluginCommand,
};
use nu_protocol::{
    Category, Example, LabeledError, Record, Signature, Span, Spanned, SyntaxShape, Value,
};
use std::collections::HashMap;
use std::path::PathBuf;

struct QueryGitPlugin;

impl Plugin for QueryGitPlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Implementation)]
    }
}
#[derive(Debug)]
struct StatementInfo {
    statement_name: String,
    // table_name and Vec<field_name>
    table_info: (String, Vec<String>, HashMap<String, String>),
}

struct Implementation;

impl SimplePluginCommand for Implementation {
    type Plugin = QueryGitPlugin;

    fn name(&self) -> &str {
        "query git"
    }

    fn usage(&self) -> &str {
        "View query git results"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .required("query", SyntaxShape::String, "GitQL query to run")
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "This is the example descripion".into(),
            example: "some pipeline involving query git".into(),
            result: None,
        }]
    }

    fn run(
        &self,
        _config: &QueryGitPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let curdir = engine.get_current_dir()?;
        let query_arg: Spanned<String> = call.req(0)?;
        let ret_val = run_gitql_query(query_arg, curdir)?;

        Ok(ret_val)
    }
}

fn main() {
    serve_plugin(&QueryGitPlugin, MsgPackSerializer);
}

fn run_gitql_query(query_arg: Spanned<String>, curdir: String) -> Result<Value, LabeledError> {
    let query = query_arg.item;
    let span = query_arg.span;
    let repository = curdir;

    // region: parameter validation
    if !std::path::Path::new(&repository).exists() {
        return Err(
            LabeledError::new(format!("path does not exist [{}]", &repository))
                .with_label("error with path", span),
        );
    }

    let metadata = std::fs::metadata(&repository).map_err(|e| {
        LabeledError::new(format!(
            "unable to get metadata for [{}], error: {}",
            &repository, e
        ))
        .with_label("error with metadata", span)
    })?;

    // This path has to be a directory
    if !metadata.is_dir() {
        return Err(
            LabeledError::new(format!("path is not a directory [{}]", &repository))
                .with_label("error with directory", span),
        );
    }

    let repo_path = match PathBuf::from(&repository).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return Err(LabeledError::new(e.to_string())
                .with_label(format!("error canonicalizing [{}]", repository), span));
        }
    };

    let mut git_repositories: Vec<git2::Repository> = vec![];
    let git_repository = git2::Repository::open(repo_path).map_err(|e| {
        LabeledError::new(e.message())
            .with_label(format!("error opening repository [{}]", repository), span)
    })?;

    // eprintln!("git_repository: {:#?}", git_repository.path());
    git_repositories.push(git_repository);
    // endregion: parameter validation

    // region: gql query

    let tokens = match tokenizer::tokenize(query) {
        Ok(t) => t,
        Err(e) => {
            return Err(LabeledError::new(format!(
                "unable to tokenize query, error: {} at: {}, {}",
                e.message, e.location.start, e.location.end
            ))
            .with_label(
                "error with tokenizer::tokenize()",
                Span::new(
                    span.start + e.location.start + 1,
                    span.start + e.location.end + 1,
                ),
            ));
        }
    };

    let statements = match parser::parse_gql(tokens) {
        Ok(p) => p,
        Err(e) => {
            return Err(LabeledError::new(format!(
                "unable to parse query, error: {} at: {}, {}",
                e.message, e.location.start, e.location.end
            ))
            .with_label(
                format!("{} error with parser::parse_gql()", e.message),
                Span::new(
                    span.start + e.location.start + 1,
                    span.start + e.location.end + 1,
                ),
            ));
        }
    };

    let mut statement_info = Vec::<StatementInfo>::new();
    statements.statements.iter().for_each(|s| {
        statement_info.push(StatementInfo {
            statement_name: s.0.to_string(),
            table_info: match s.1.get_statement_kind() {
                StatementKind::Select => {
                    let st = s.1;
                    let st = match st.as_any().downcast_ref::<SelectStatement>() {
                        Some(st) => {
                            // eprintln!(
                            //     "select_stmt:\nalias: {:?}\ntable_name: {}\nfield_names: {:#?}\nis_distinct: {}",
                            //     st.alias_table, st.table_name, st.fields_names, st.is_distinct,
                            // );
                            (
                                st.table_name.to_string(),
                                st.fields_names.clone(),
                                st.alias_table.clone(),
                            )
                        }
                        None => panic!("downcast failed"),
                    };
                    st
                }
                StatementKind::Where => ("Where".into(), vec![], HashMap::new()),
                StatementKind::Having => ("Having".into(), vec![], HashMap::new()),
                StatementKind::Limit => ("Limit".into(), vec![], HashMap::new()),
                StatementKind::Offset => ("Offset".into(), vec![], HashMap::new()),
                StatementKind::OrderBy => ("OrderBy".into(), vec![], HashMap::new()),
                StatementKind::GroupBy => ("GroupBy".into(), vec![], HashMap::new()),
                StatementKind::AggregateFunction => {
                    let af = s.1;
                    let af = match af.as_any().downcast_ref::<AggregationFunctionsStatement>() {
                        Some(af) => {
                            // eprintln!(
                            //     "AggregateFunctionStatement:\naggregations: {:?}",
                            //     af.aggregations
                            //         .iter()
                            //         .map(|(x, y)| {
                            //             (
                            //                 x.to_string(),
                            //                 format!("{}|{}", y.function_name, y.argument),
                            //             )
                            //         })
                            //         .collect::<Vec<_>>(),
                            // );
                            (
                                "AggregateFunction".into(),
                                vec![],
                                af.aggregations
                                    .iter()
                                    .map(|(x, y)| {
                                        (
                                            x.to_string(),
                                            format!("{}|{}", y.function_name, y.argument),
                                        )
                                    })
                                    .collect::<HashMap<_, _>>(),
                            )
                        }
                        None => panic!("downcast failed"),
                    };
                    af
                }
            },
        });
    });
    // eprintln!("statement_info: {:#?}", statement_info);
    let evaluation_result = engine::evaluate(&git_repositories, statements);
    // Report Runtime exceptions if they exists
    if evaluation_result.is_err() {
        // reporter.report_runtime_error(evaluation_result.err().unwrap());
        // input.clear();
        // continue;
    }

    let mut evaluation_values = evaluation_result.ok().unwrap();
    // let out_val = render_objects(
    //     &mut evaluation_values.groups,
    //     &evaluation_values.hidden_selections,
    //     // false,
    //     // 500,
    // );

    // endregion: gql query

    // region: gql output to nushell values
    let out_val = render_objects2(
        &mut evaluation_values.groups,
        // &evaluation_values.hidden_selections,
        statement_info,
    );

    // let debug = true;
    // if debug {
    //     eprintln!("\n");
    //     eprintln!("Analysis:");
    //     eprintln!("Frontend : {:?}", front_duration);
    //     eprintln!("Engine   : {:?}", engine_duration);
    //     eprintln!("Total    : {:?}", (front_duration + engine_duration));
    //     eprintln!("\n");
    // }

    // endregion: gql output to nushell values

    Ok(out_val)
}

fn render_objects2(
    groups: &mut Vec<Vec<GQLObject>>,
    // hidden_selections: &[String],
    stmt_info: Vec<StatementInfo>,
) -> Value {
    // eprintln!("render_objects2");
    // eprintln!("groups.len(): {:#?}", groups.len());

    let mut table_info = ("".to_string(), vec![], HashMap::new());
    for t in stmt_info {
        if t.statement_name == "select" {
            table_info = t.table_info;
            break;
        }
    }
    // eprintln!("table_info: {:#?}", table_info);
    if groups.len() > 1 {
        // for x in groups.clone() {
        //     for y in x {
        //         for a in y.attributes.clone() {
        //             eprintln!("a.0: {:#?} a.1: {:#?}", a.0, a.1.literal());
        //         }
        //     }
        // }
        flat_gql_groups(groups);
    }

    if groups.is_empty() || groups[0].is_empty() {
        return Value::test_nothing();
    }

    // let gql_group = groups.first().unwrap();
    // let gql_group_len = gql_group.len();

    // let titles: Vec<&str> = groups[0][0]
    //     .attributes
    //     .keys()
    //     .filter(|s| !hidden_selections.contains(s))
    //     .map(|k| k.as_ref())
    //     .collect();

    // References table
    // Name	        Type	Description
    // name	        Text	Reference name
    // full_name	Text	Reference full name
    // type	        Text	Reference type
    // repo	        Text	Repository full path

    // Commits table
    // Name	        Type	Description
    // commit_id	Text	Commit id
    // title	    Text	Commit title
    // message	    Text	Commit full message
    // name	        Text	Author name
    // email	    Text	Author email
    // datetime	    Date	Commit date time
    // repo	        Text	Repository full path

    // Diffs table
    // Name	            Type	Description
    // commit_id	    Text	Commit id
    // name	            Text	Author name
    // email	        Text	Author email
    // insertions	    Number	Number of inserted lines
    // deletions	    Number	Number of deleted lines
    // files_changed	Number	Number of file changed
    // repo	            Text	Repository full path

    // Branches table
    // Name	        Type	    Description
    // name	        Text	    Branch name
    // commit_count	Number	    Number of commits in this branch
    // is_head	    Bool	    Is the head branch
    // is_remote	Bool	    Is a remote branch
    // repo	        Text	    Repository full path

    // Tags table
    // Name	    Type	Description
    // name	    Text	Tag name
    // repo	    Text	Repository full path

    let mut recs = vec![];
    for a in groups[0].clone() {
        let mut rec = Record::new();
        // for x in a.attributes.clone() {
        //     eprintln!("x.0: {:#?} x.1: {:#?}", x.0, x.1.literal());
        // }
        // if table_name == "commits"
        match table_info.0.as_str() {
            "refs" | "references" => {
                if let Some((rec_str, rec_val)) =
                    get_column_record("name", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("full_name", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("type", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("repo", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                let mut the_rest = table_info.1.clone();
                let standard_columns = [
                    "name".to_string(),
                    "full_name".to_string(),
                    "type".to_string(),
                    "repo".to_string(),
                ];
                the_rest.retain(|x| !standard_columns.contains(x));
                if !the_rest.is_empty() {
                    for x in the_rest {
                        if let Some((rec_str, rec_val)) =
                            get_column_record(&x, table_info.clone(), &a, "str")
                        {
                            rec.push(rec_str, rec_val);
                        } else {
                            rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                        }
                    }
                }
            }

            "commits" => {
                if let Some((rec_str, rec_val)) =
                    get_column_record("commit_id", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("title", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("message", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("name", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("email", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("datetime", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("repo", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                let mut the_rest = table_info.1.clone();
                let standard_columns = [
                    "commit_id".to_string(),
                    "title".to_string(),
                    "message".to_string(),
                    "name".to_string(),
                    "email".to_string(),
                    "datetime".to_string(),
                    "repo".to_string(),
                ];
                the_rest.retain(|x| !standard_columns.contains(x));
                if !the_rest.is_empty() {
                    for x in the_rest {
                        if let Some((rec_str, rec_val)) =
                            get_column_record(&x, table_info.clone(), &a, "str")
                        {
                            rec.push(rec_str, rec_val);
                        } else {
                            rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                        }
                    }
                }
            }

            "diffs" => {
                if let Some((rec_str, rec_val)) =
                    get_column_record("commit_id", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("name", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("email", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("insertions", table_info.clone(), &a, "int")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("deletions", table_info.clone(), &a, "int")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("files_changed", table_info.clone(), &a, "int")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("repo", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                let mut the_rest = table_info.1.clone();
                let standard_columns = [
                    "commit_id".to_string(),
                    "name".to_string(),
                    "email".to_string(),
                    "insertions".to_string(),
                    "deletions".to_string(),
                    "files_changed".to_string(),
                    "repo".to_string(),
                ];
                the_rest.retain(|x| !standard_columns.contains(x));
                if !the_rest.is_empty() {
                    for x in the_rest {
                        if let Some((rec_str, rec_val)) =
                            get_column_record(&x, table_info.clone(), &a, "str")
                        {
                            rec.push(rec_str, rec_val);
                        } else {
                            rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                        }
                    }
                }
            }

            "branches" => {
                if let Some((rec_str, rec_val)) =
                    get_column_record("name", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("commit_count", table_info.clone(), &a, "int")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("is_head", table_info.clone(), &a, "bool")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("is_remote", table_info.clone(), &a, "bool")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("repo", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                let mut the_rest = table_info.1.clone();
                let standard_columns = [
                    "name".to_string(),
                    "commit_count".to_string(),
                    "is_head".to_string(),
                    "is_remote".to_string(),
                    "repo".to_string(),
                ];
                the_rest.retain(|x| !standard_columns.contains(x));
                if !the_rest.is_empty() {
                    for x in the_rest {
                        if let Some((rec_str, rec_val)) =
                            get_column_record(&x, table_info.clone(), &a, "str")
                        {
                            rec.push(rec_str, rec_val);
                        } else {
                            rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                        }
                    }
                }
            }

            "tags" => {
                if let Some((rec_str, rec_val)) =
                    get_column_record("name", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                if let Some((rec_str, rec_val)) =
                    get_column_record("repo", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }

                let mut the_rest = table_info.1.clone();
                let standard_columns = ["name".to_string(), "repo".to_string()];
                the_rest.retain(|x| !standard_columns.contains(x));
                if !the_rest.is_empty() {
                    for x in the_rest {
                        if let Some((rec_str, rec_val)) =
                            get_column_record(&x, table_info.clone(), &a, "str")
                        {
                            rec.push(rec_str, rec_val);
                        } else {
                            rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                        }
                    }
                }
            }
            _ => {
                if !table_info.1.clone().is_empty() {
                    for x in table_info.1.clone() {
                        rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                    }
                }
            }
        }
        recs.push(Value::test_record(rec));
    }

    Value::test_list(recs)
}

fn get_column_record(
    lookup: &str,
    table_info: (String, Vec<String>, HashMap<String, String>),
    gqlobj: &GQLObject,
    output_type: &str,
) -> Option<(String, Value)> {
    // table_info.1 is the column name
    // table_info.2 is the hashmap column_name: column_alias
    if table_info.1.contains(&lookup.to_string()) {
        if table_info.2.contains_key(lookup) {
            // eprintln!("in table_info.2 {}", lookup);
            let table_alias = table_info.2[lookup].clone();
            let (rec_s, rec_v) = if output_type == "str" {
                let rec_str = table_alias.to_string();
                let rec_val = Value::test_string(gqlobj.attributes[&table_alias].literal());
                (rec_str.to_string(), rec_val)
            } else if output_type == "int" {
                let rec_str = table_alias.to_string();
                let rec_val = Value::test_int(gqlobj.attributes[&table_alias].as_int());
                (rec_str, rec_val)
            } else if output_type == "bool" {
                let rec_str = table_alias.to_string();
                let rec_val = Value::test_bool(gqlobj.attributes[&table_alias].as_bool());
                (rec_str, rec_val)
            } else {
                ("".to_string(), Value::test_nothing())
            };
            return Some((rec_s, rec_v));
        } else {
            // eprintln!("not in table_info.2 {}", lookup);
            let (rec_s, rec_v) = if output_type == "str" {
                let rec_str = lookup.to_string();
                let rec_val = Value::test_string(gqlobj.attributes[lookup].literal());
                (rec_str.to_string(), rec_val)
            } else if output_type == "int" {
                let rec_str = lookup.to_string();
                let rec_val = Value::test_int(gqlobj.attributes[lookup].as_int());
                (rec_str, rec_val)
            } else if output_type == "bool" {
                let rec_str = lookup.to_string();
                let rec_val = Value::test_bool(gqlobj.attributes[lookup].as_bool());
                (rec_str, rec_val)
            } else {
                ("".to_string(), Value::test_nothing())
            };
            return Some((rec_s, rec_v));
        }
    } else {
        None
    }
}
