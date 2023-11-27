use gitql_ast::object::flat_gql_groups;
use gitql_ast::object::GQLObject;
use gitql_engine::engine;
use gitql_parser::parser;
use gitql_parser::tokenizer;
use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{
    record, Category, PluginExample, PluginSignature, Record, Spanned, SyntaxShape, Value,
};
use std::path::PathBuf;
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

    eprintln!("git_repository: {:#?}", git_repository.path());
    git_repositories.push(git_repository);

    let front_start = std::time::Instant::now();
    let tokenizer_result = tokenizer::tokenize(query);
    if tokenizer_result.is_err() {
        // reporter.report_gql_error(tokenizer_result.err().unwrap());
        // input.clear();
        // continue;
    }

    let tokens = tokenizer_result.ok().unwrap();
    let parser_result = parser::parse_gql(tokens);
    // eprintln!("parser_result: {:#?}", parser_result);
    if parser_result.is_err() {
        // reporter.report_gql_error(parser_result.err().unwrap());
        // input.clear();
        // continue;
    }

    let statements = parser_result.ok().unwrap();
    let front_duration = front_start.elapsed();

    let engine_start = std::time::Instant::now();
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

    let out_val = render_objects2(
        &mut evaluation_values.groups,
        &evaluation_values.hidden_selections,
    );

    let engine_duration = engine_start.elapsed();

    let debug = true;
    if debug {
        eprintln!("\n");
        eprintln!("Analysis:");
        eprintln!("Frontend : {:?}", front_duration);
        eprintln!("Engine   : {:?}", engine_duration);
        eprintln!("Total    : {:?}", (front_duration + engine_duration));
        eprintln!("\n");
    }

    // let output = String::new();

    Ok(out_val)
}

fn render_objects(
    groups: &mut Vec<Vec<GQLObject>>,
    hidden_selections: &[String],
    // pagination: bool,
    // page_size: usize,
) -> Value {
    eprintln!("groups.len(): {:#?}", groups.len());
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

    let gql_group = groups.first().unwrap();
    // let gql_group_len = gql_group.len();

    let titles: Vec<&str> = groups[0][0]
        .attributes
        .keys()
        .filter(|s| !hidden_selections.contains(s))
        .map(|k| k.as_ref())
        .collect();

    for x in groups[0].clone() {
        for y in x.attributes.clone() {
            eprintln!("key: {:#?} value: {:#?}", y.0, y.1.literal());
        }
    }
    // Setup table headers
    // let header_color = comfy_table::Color::Green;
    let mut table_headers = vec![];
    for key in &titles {
        // table_headers.push(comfy_table::Cell::new(key).fg(header_color));
        table_headers.push(key);
    }

    // Print all data without pagination
    // if !pagination || page_size >= gql_group_len {
    //     return print_group_as_table(&titles, table_headers, gql_group);
    //     // Value::test_nothing()
    // }
    print_group_as_table(&titles, table_headers, gql_group)

    // // Setup the pagination mode
    // let number_of_pages = (gql_group_len as f64 / page_size as f64).ceil() as usize;
    // let current_page = 1;

    // loop {
    //     let start_index = (current_page - 1) * page_size;
    //     let end_index = (start_index + page_size).min(gql_group_len);

    //     let current_page_groups = &gql_group[start_index..end_index].to_vec();

    //     eprintln!("Page {}/{}", current_page, number_of_pages);
    //     return print_group_as_table(&titles, table_headers.clone(), current_page_groups);

    //     // let pagination_input = handle_pagination_input(current_page, number_of_pages);
    //     // match pagination_input {
    //     //     PaginationInput::NextPage => current_page += 1,
    //     //     PaginationInput::PreviousPage => current_page -= 1,
    //     //     PaginationInput::Quit => break,
    //     // }
    // }
}

fn render_objects2(groups: &mut Vec<Vec<GQLObject>>, hidden_selections: &[String]) -> Value {
    eprintln!("render_objects2");
    eprintln!("groups.len(): {:#?}", groups.len());
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

    let gql_group = groups.first().unwrap();
    // let gql_group_len = gql_group.len();

    // let titles: Vec<&str> = groups[0][0]
    //     .attributes
    //     .keys()
    //     .filter(|s| !hidden_selections.contains(s))
    //     .map(|k| k.as_ref())
    //     .collect();

    let mut recs = vec![];
    for a in groups[0].clone() {
        let mut rec = Record::new();
        for x in a.attributes.clone() {
            eprintln!("x.0: {:#?} x.1: {:#?}", x.0, x.1.literal());
            rec.push(x.0, Value::test_string(x.1.literal()));
        }
        recs.push(Value::test_record(rec));
    }
    eprintln!("rec: {:#?}", recs.clone());
    // Value::test_nothing()
    Value::test_list(recs)
}

fn print_group_as_table(
    titles: &Vec<&str>,
    table_headers: Vec<&&str>,
    group: &Vec<GQLObject>,
) -> Value {
    eprintln!("titles: {:#?}", titles);
    eprintln!("table_headers: {:#?}", table_headers);

    let mut table = vec![];

    let header_length = table_headers.len();
    // Add rows to the table
    for object in group {
        let mut table_row = vec![];
        for (idx, key) in titles.iter().enumerate() {
            let lookup = idx % header_length;
            let value = object.attributes.get(&key.to_string()).unwrap();
            let value_literal = value.literal();
            table_row.push((titles[lookup].to_string(), value_literal));
        }
        table.push(table_row);
    }

    let mut rec_list = vec![];

    for row in &table {
        let mut rec = Record::new();

        for (head, val) in row {
            rec.push(head, Value::test_string(val))
        }
        rec_list.push(Value::test_record(rec));
    }

    // Print table
    eprintln!("table: {:#?}", table);

    Value::test_list(rec_list)
}
