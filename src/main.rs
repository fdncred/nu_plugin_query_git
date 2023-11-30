use gitql_ast::object::flat_gql_groups;
use gitql_ast::object::GQLObject;
use gitql_ast::statement::SelectStatement;
use gitql_ast::statement::StatementKind;
use gitql_engine::engine;
use gitql_parser::parser;
use gitql_parser::tokenizer;
use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{
    Category, PluginExample, PluginSignature, Record, Span, Spanned, SyntaxShape, Value,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
struct StatementInfo {
    statement_name: String,
    // table_name and Vec<field_name>
    table_info: (String, Vec<String>, HashMap<String, String>),
}

struct Implementation;

impl Implementation {
    fn new() -> Self {
        Self {}
    }
}

impl Plugin for Implementation {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("query git")
            .usage("View query git results")
            .required("query", SyntaxShape::String, "GitQL query to run")
            .category(Category::Experimental)
            .plugin_examples(vec![PluginExample {
                description: "This is the example descripion".into(),
                example: "some pipeline involving query git".into(),
                result: None,
            }])]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        assert_eq!(name, "query git");
        let query_arg: Spanned<String> = call.req(0)?;

        let ret_val = run_gitql_query(query_arg)?;
        //         return Err(LabeledError {
        //             label: "Expected something from pipeline".into(),
        //             msg: format!("requires some input, got {}", v.get_type()),
        //             span: Some(call.head),
        //         });

        Ok(ret_val)
    }
}

fn main() {
    serve_plugin(&mut Implementation::new(), MsgPackSerializer);
}

fn run_gitql_query(query_arg: Spanned<String>) -> Result<Value, LabeledError> {
    let query = query_arg.item;
    let span = query_arg.span;
    let repository = ".";

    // region: parameter validation
    if !std::path::Path::new(&repository).exists() {
        return Err(LabeledError {
            label: "error with path".to_string(),
            msg: format!("path does not exist [{}]", &repository),
            span: Some(span),
        });
    }

    let metadata = std::fs::metadata(&repository).map_err(|e| LabeledError {
        label: "error with metadata".to_string(),
        msg: format!("unable to get metadata for [{}], error: {}", &repository, e),
        span: Some(span),
    })?;

    // This path has to be a directory
    if !metadata.is_dir() {
        return Err(LabeledError {
            label: "error with directory".to_string(),
            msg: format!("path is not a directory [{}]", &repository),
            span: Some(span),
        });
    }

    let repo_path = match PathBuf::from(&repository).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return Err(LabeledError {
                label: format!("error canonicalizing [{}]", repository),
                msg: e.to_string(),
                span: Some(span),
            });
        }
    };

    let mut git_repositories: Vec<git2::Repository> = vec![];
    let git_repository = git2::Repository::open(repo_path).map_err(|e| LabeledError {
        label: format!("error opening repository [{}]", repository),
        msg: e.message().to_string(),
        span: Some(span),
    })?;

    // eprintln!("git_repository: {:#?}", git_repository.path());
    git_repositories.push(git_repository);
    // endregion: parameter validation

    // region: gql query

    let tokens = match tokenizer::tokenize(query) {
        Ok(t) => t,
        Err(e) => {
            return Err(LabeledError {
                label: "error with tokenizer::tokenize()".to_string(),
                msg: format!(
                    "unable to tokenize query, error: {} at: {}, {}",
                    e.message, e.location.start, e.location.end
                ),
                span: Some(Span::new(
                    span.start + e.location.start + 1,
                    span.start + e.location.end + 1,
                )),
            });
        }
    };

    let statements = match parser::parse_gql(tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("span: {:#?}", span);
            return Err(LabeledError {
                label: format!("{} error with parser::parse_gql()", e.message),
                msg: format!(
                    "unable to parse query, error: {} at: {}, {}",
                    e.message, e.location.start, e.location.end
                ),
                span: Some(Span::new(
                    span.start + e.location.start + 1,
                    span.start + e.location.end + 1,
                )),
            });
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
                            //     st.alias_table, st.table_name, st.fields_names, st.is_distinct
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
                    ("AggregateFunction".into(), vec![], HashMap::new())
                }
            },
        });
    });
    // eprintln!("statement_info: {:#?}", statement_info);
    // let front_duration = front_start.elapsed();

    // let engine_start = std::time::Instant::now();
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

    // let engine_duration = engine_start.elapsed();

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

    // let output = String::new();

    Ok(out_val)
}

// region: dead code
// fn render_objects(
//     groups: &mut Vec<Vec<GQLObject>>,
//     hidden_selections: &[String],
//     // pagination: bool,
//     // page_size: usize,
// ) -> Value {
//     eprintln!("groups.len(): {:#?}", groups.len());
//     if groups.len() > 1 {
//         // for x in groups.clone() {
//         //     for y in x {
//         //         for a in y.attributes.clone() {
//         //             eprintln!("a.0: {:#?} a.1: {:#?}", a.0, a.1.literal());
//         //         }
//         //     }
//         // }
//         flat_gql_groups(groups);
//     }

//     if groups.is_empty() || groups[0].is_empty() {
//         return Value::test_nothing();
//     }

//     let gql_group = groups.first().unwrap();
//     // let gql_group_len = gql_group.len();

//     let titles: Vec<&str> = groups[0][0]
//         .attributes
//         .keys()
//         .filter(|s| !hidden_selections.contains(s))
//         .map(|k| k.as_ref())
//         .collect();

//     for x in groups[0].clone() {
//         for y in x.attributes.clone() {
//             eprintln!("key: {:#?} value: {:#?}", y.0, y.1.literal());
//         }
//     }
//     // Setup table headers
//     // let header_color = comfy_table::Color::Green;
//     let mut table_headers = vec![];
//     for key in &titles {
//         // table_headers.push(comfy_table::Cell::new(key).fg(header_color));
//         table_headers.push(key);
//     }

//     // Print all data without pagination
//     // if !pagination || page_size >= gql_group_len {
//     //     return print_group_as_table(&titles, table_headers, gql_group);
//     //     // Value::test_nothing()
//     // }
//     print_group_as_table(&titles, table_headers, gql_group)

//     // // Setup the pagination mode
//     // let number_of_pages = (gql_group_len as f64 / page_size as f64).ceil() as usize;
//     // let current_page = 1;

//     // loop {
//     //     let start_index = (current_page - 1) * page_size;
//     //     let end_index = (start_index + page_size).min(gql_group_len);

//     //     let current_page_groups = &gql_group[start_index..end_index].to_vec();

//     //     eprintln!("Page {}/{}", current_page, number_of_pages);
//     //     return print_group_as_table(&titles, table_headers.clone(), current_page_groups);

//     //     // let pagination_input = handle_pagination_input(current_page, number_of_pages);
//     //     // match pagination_input {
//     //     //     PaginationInput::NextPage => current_page += 1,
//     //     //     PaginationInput::PreviousPage => current_page -= 1,
//     //     //     PaginationInput::Quit => break,
//     //     // }
//     // }
// }
// endregion: dead code

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
    eprintln!("table_info: {:#?}", table_info);
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
                if table_info.1.contains(&"name".to_string()) {
                    rec.push("name", Value::test_string(a.attributes["name"].literal()));
                }
                if table_info.1.contains(&"full_name".to_string()) {
                    rec.push(
                        "full_name",
                        Value::test_string(a.attributes["full_name"].literal()),
                    );
                }
                if table_info.1.contains(&"type".to_string()) {
                    rec.push("type", Value::test_string(a.attributes["type"].literal()));
                }
                if table_info.1.contains(&"repo".to_string()) {
                    rec.push("repo", Value::test_string(a.attributes["repo"].literal()));
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
                        rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                    }
                }
            }

            "commits" => {
                // if table_info.1.contains(&"commit_id".to_string()) {
                //     rec.push(
                //         "commit_id",
                //         Value::test_string(a.attributes["commit_id"].literal()),
                //     );
                // }
                if let Some((rec_str, rec_val)) =
                    get_column_record("commit_id", table_info.clone(), &a, "str")
                {
                    rec.push(rec_str, rec_val);
                }
                if table_info.1.contains(&"title".to_string()) {
                    if table_info.2.contains_key("title") {
                        let table_alias = table_info.2["title"].clone();
                        rec.push(
                            &table_alias,
                            Value::test_string(a.attributes[&table_alias].literal()),
                        );
                    } else {
                        rec.push("title", Value::test_string(a.attributes["title"].literal()));
                    }
                }
                if table_info.1.contains(&"message".to_string()) {
                    rec.push(
                        "message",
                        Value::test_string(a.attributes["message"].literal()),
                    );
                }
                if table_info.1.contains(&"name".to_string()) {
                    rec.push("name", Value::test_string(a.attributes["name"].literal()));
                }
                if table_info.1.contains(&"email".to_string()) {
                    rec.push("email", Value::test_string(a.attributes["email"].literal()));
                }
                if table_info.1.contains(&"datetime".to_string()) {
                    rec.push(
                        "datetime",
                        Value::test_string(a.attributes["datetime"].literal()),
                    );
                }
                if table_info.1.contains(&"repo".to_string()) {
                    rec.push("repo", Value::test_string(a.attributes["repo"].literal()));
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
                        rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                    }
                }
            }

            "diffs" => {
                if table_info.1.contains(&"commit_id".to_string()) {
                    rec.push(
                        "commit_id",
                        Value::test_string(a.attributes["commit_id"].literal()),
                    );
                }
                if table_info.1.contains(&"name".to_string()) {
                    rec.push("name", Value::test_string(a.attributes["name"].literal()));
                }
                if table_info.1.contains(&"email".to_string()) {
                    rec.push("email", Value::test_string(a.attributes["email"].literal()));
                }
                if table_info.1.contains(&"insertions".to_string()) {
                    rec.push(
                        "insertions",
                        Value::test_int(a.attributes["insertions"].as_int()),
                    );
                }
                if table_info.1.contains(&"deletions".to_string()) {
                    rec.push(
                        "deletions",
                        Value::test_int(a.attributes["deletions"].as_int()),
                    );
                }
                if table_info.1.contains(&"files_changed".to_string()) {
                    rec.push(
                        "files_changed",
                        Value::test_int(a.attributes["files_changed"].as_int()),
                    );
                }
                if table_info.1.contains(&"repo".to_string()) {
                    rec.push("repo", Value::test_string(a.attributes["repo"].literal()));
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
                        rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                    }
                }
            }

            "branches" => {
                if table_info.1.contains(&"name".to_string()) {
                    rec.push("name", Value::test_string(a.attributes["name"].literal()));
                }
                if table_info.1.contains(&"commit_count".to_string()) {
                    rec.push(
                        "commit_count",
                        Value::test_int(a.attributes["commit_count"].as_int()),
                    );
                }
                if table_info.1.contains(&"is_head".to_string()) {
                    rec.push(
                        "is_head",
                        Value::test_bool(a.attributes["is_head"].as_bool()),
                    );
                }
                if table_info.1.contains(&"is_remote".to_string()) {
                    rec.push(
                        "is_remote",
                        Value::test_bool(a.attributes["is_remote"].as_bool()),
                    );
                }
                if table_info.1.contains(&"repo".to_string()) {
                    rec.push("repo", Value::test_string(a.attributes["repo"].literal()));
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
                        rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
                    }
                }
            }
            "tags" => {
                if table_info.1.contains(&"name".to_string()) {
                    rec.push("name", Value::test_string(a.attributes["name"].literal()));
                }
                if table_info.1.contains(&"repo".to_string()) {
                    rec.push("repo", Value::test_string(a.attributes["repo"].literal()));
                }
                let mut the_rest = table_info.1.clone();
                let standard_columns = ["name".to_string(), "repo".to_string()];
                the_rest.retain(|x| !standard_columns.contains(x));
                if !the_rest.is_empty() {
                    for x in the_rest {
                        rec.push(x.clone(), Value::test_string(a.attributes[&x].literal()));
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
    // eprintln!("rec: {:#?}", recs.clone());
    // Value::test_nothing()
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
// fn print_group_as_table(
//     titles: &Vec<&str>,
//     table_headers: Vec<&&str>,
//     group: &Vec<GQLObject>,
// ) -> Value {
//     eprintln!("titles: {:#?}", titles);
//     eprintln!("table_headers: {:#?}", table_headers);

//     let mut table = vec![];

//     let header_length = table_headers.len();
//     // Add rows to the table
//     for object in group {
//         let mut table_row = vec![];
//         for (idx, key) in titles.iter().enumerate() {
//             let lookup = idx % header_length;
//             let value = object.attributes.get(&key.to_string()).unwrap();
//             let value_literal = value.literal();
//             table_row.push((titles[lookup].to_string(), value_literal));
//         }
//         table.push(table_row);
//     }

//     let mut rec_list = vec![];

//     for row in &table {
//         let mut rec = Record::new();

//         for (head, val) in row {
//             rec.push(head, Value::test_string(val))
//         }
//         rec_list.push(Value::test_record(rec));
//     }

//     // Print table
//     eprintln!("table: {:#?}", table);

//     Value::test_list(rec_list)
// }
