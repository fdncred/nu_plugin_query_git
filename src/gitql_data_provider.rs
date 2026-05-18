use gitql_core::object::Row;
use gitql_core::values::{
    boolean::BoolValue, datetime::DateTimeValue, integer::IntValue, null::NullValue,
    text::TextValue, Value,
};
use gitql_engine::data_provider::DataProvider;
use gix::refs::Category;
use std::path::Path;

/// GitQL data provider backed by one or more local Git repositories.
pub struct GitDataProvider {
    pub repos: Vec<gix::Repository>,
}

impl GitDataProvider {
    pub fn new(repos: Vec<gix::Repository>) -> Self {
        Self { repos }
    }
}

impl DataProvider for GitDataProvider {
    fn provide(&self, table: &str, selected_columns: &[String]) -> Result<Vec<Row>, String> {
        let mut rows: Vec<Row> = Vec::new();

        for repository in &self.repos {
            rows.extend(select_gql_objects(repository, table, selected_columns)?);
        }

        Ok(rows)
    }
}

fn repo_workdir_path(repo: &gix::Repository) -> String {
    repo.workdir()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| repo.path().to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn repo_name_from_path(repo_path: &str) -> String {
    Path::new(repo_path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or(repo_path)
        .to_string()
}

fn text_value(value: impl ToString) -> Box<dyn Value> {
    Box::new(TextValue {
        value: value.to_string(),
    })
}

fn int_value(value: i64) -> Box<dyn Value> {
    Box::new(IntValue { value })
}

fn bool_value(value: bool) -> Box<dyn Value> {
    Box::new(BoolValue { value })
}

fn null_value() -> Box<dyn Value> {
    Box::new(NullValue)
}

fn repo_metadata_value(
    column_name: &str,
    repo_path: &str,
    repo_name: &str,
) -> Option<Box<dyn Value>> {
    match column_name {
        "repo" => Some(text_value(repo_path)),
        "repo_name" => Some(text_value(repo_name)),
        _ => None,
    }
}

fn select_gql_objects(
    repo: &gix::Repository,
    table: &str,
    selected_columns: &[String],
) -> Result<Vec<Row>, String> {
    match table {
        "refs" => select_references(repo, selected_columns),
        "commits" => select_commits(repo, selected_columns),
        "branches" => select_branches(repo, selected_columns),
        "diffs" => select_diffs(repo, selected_columns),
        "tags" => select_tags(repo, selected_columns),
        _ => Ok(vec![Row { values: vec![] }]),
    }
}

fn select_references(
    repo: &gix::Repository,
    selected_columns: &[String],
) -> Result<Vec<Row>, String> {
    let references = repo.references().map_err(|err| err.to_string())?;
    let repo_path = repo_workdir_path(repo);
    let repo_name = repo_name_from_path(&repo_path);
    let mut rows: Vec<Row> = Vec::new();

    let reference_iter = references.all().map_err(|err| err.to_string())?;
    for reference in reference_iter.flatten() {
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());
        for column_name in selected_columns {
            let column_name = column_name.as_str();
            let value = match column_name {
                "name" => text_value(
                    reference
                        .name()
                        .category_and_short_name()
                        .map(|(_, sn)| sn.to_string())
                        .unwrap_or_default(),
                ),
                "full_name" => text_value(reference.name().as_bstr().to_string()),
                "type" => {
                    let category = reference.name().category();
                    let reference_type = if matches!(category, Some(Category::LocalBranch)) {
                        "branch"
                    } else if matches!(category, Some(Category::RemoteBranch)) {
                        "remote"
                    } else if matches!(category, Some(Category::Tag)) {
                        "tag"
                    } else if matches!(category, Some(Category::Note)) {
                        "note"
                    } else {
                        "other"
                    };
                    text_value(reference_type)
                }
                _ => repo_metadata_value(column_name, &repo_path, &repo_name)
                    .unwrap_or_else(null_value),
            };
            values.push(value);
        }

        rows.push(Row { values });
    }

    Ok(rows)
}

fn select_commits(repo: &gix::Repository, selected_columns: &[String]) -> Result<Vec<Row>, String> {
    let head_id = repo.head_id().map_err(|err| err.to_string())?;
    let repo_path = repo_workdir_path(repo);
    let repo_name = repo_name_from_path(&repo_path);
    let revwalk = head_id.ancestors().all().map_err(|err| err.to_string())?;
    let mut rows: Vec<Row> = Vec::new();

    for commit_info in revwalk {
        let commit_info = commit_info.map_err(|err| err.to_string())?;
        let commit = repo
            .find_object(commit_info.id)
            .map_err(|err| err.to_string())?
            .into_commit();
        let commit = commit.decode().map_err(|err| err.to_string())?;

        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());
        for column_name in selected_columns {
            let column_name = column_name.as_str();
            let value = match column_name {
                "commit_id" => text_value(commit_info.id.to_string()),
                "author_name" => text_value(
                    commit
                        .author()
                        .map(|author| author.name.to_string())
                        .unwrap_or_default(),
                ),
                "author_email" => text_value(
                    commit
                        .author()
                        .map(|author| author.email.to_string())
                        .unwrap_or_default(),
                ),
                "committer_name" => text_value(
                    commit
                        .committer()
                        .map(|committer| committer.name.to_string())
                        .unwrap_or_default(),
                ),
                "committer_email" => text_value(
                    commit
                        .committer()
                        .map(|committer| committer.email.to_string())
                        .unwrap_or_default(),
                ),
                "title" => text_value(commit.message().summary().to_string()),
                "message" => text_value(commit.message.to_string()),
                "datetime" => Box::new(DateTimeValue {
                    value: commit_info
                        .commit_time
                        .unwrap_or_else(|| commit.time().map(|time| time.seconds).unwrap_or(0)),
                }),
                "parents_count" => int_value(commit.parents.len() as i64),
                _ => repo_metadata_value(column_name, &repo_path, &repo_name)
                    .unwrap_or_else(null_value),
            };
            values.push(value);
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}

fn select_branches(
    repo: &gix::Repository,
    selected_columns: &[String],
) -> Result<Vec<Row>, String> {
    let mut rows: Vec<Row> = vec![];

    let repo_path = repo_workdir_path(repo);
    let repo_name = repo_name_from_path(&repo_path);
    let platform = repo.references().map_err(|err| err.to_string())?;
    let local_branches = platform.local_branches().map_err(|err| err.to_string())?;
    let remote_branches = platform.remote_branches().map_err(|err| err.to_string())?;
    let local_and_remote_branches = local_branches.chain(remote_branches);
    let head_ref = match repo.head_ref().map_err(|err| err.to_string())? {
        Some(head_ref) => head_ref,
        None => return Ok(rows),
    };

    for mut branch in local_and_remote_branches.flatten() {
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());

        for column_name in selected_columns {
            let column_name = column_name.as_str();
            let value = match column_name {
                "name" => text_value(branch.name().as_bstr().to_string()),
                "commit_count" => int_value(
                    branch
                        .try_id()
                        .and_then(|id| id.ancestors().all().ok())
                        .map_or(-1, |revwalk| revwalk.count() as i64),
                ),
                "updated" => {
                    let timestamp = branch
                        .peel_to_id()
                        .ok()
                        .and_then(|id| id.ancestors().all().ok())
                        .and_then(|mut revwalk| revwalk.next())
                        .and_then(|commit_info| commit_info.ok())
                        .and_then(|commit_info| {
                            commit_info.commit_time.or_else(|| {
                                commit_info.object().ok().and_then(|object| {
                                    object.decode().ok().and_then(|commit| {
                                        commit.time().ok().map(|time| time.seconds)
                                    })
                                })
                            })
                        });

                    if let Some(time_stamp) = timestamp {
                        Box::new(DateTimeValue { value: time_stamp })
                    } else {
                        null_value()
                    }
                }
                "is_head" => bool_value(branch.inner == head_ref.inner),
                "is_remote" => bool_value(matches!(
                    branch.name().category(),
                    Some(Category::RemoteBranch)
                )),
                _ => repo_metadata_value(column_name, &repo_path, &repo_name)
                    .unwrap_or_else(null_value),
            };
            values.push(value);
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}

fn select_diffs(repo: &gix::Repository, selected_columns: &[String]) -> Result<Vec<Row>, String> {
    let mut repo = repo.clone();
    repo.object_cache_size_if_unset(4 * 1024 * 1024);

    let revwalk = repo
        .head_id()
        .map_err(|err| err.to_string())?
        .ancestors()
        .all()
        .map_err(|err| err.to_string())?;
    let repo_path = repo_workdir_path(&repo);
    let repo_name = repo_name_from_path(&repo_path);

    let mut rewrite_cache = repo
        .diff_resource_cache(gix::diff::blob::pipeline::Mode::ToGit, Default::default())
        .map_err(|err| err.to_string())?;

    let mut diff_cache = rewrite_cache.clone();
    let mut rows: Vec<Row> = vec![];

    let select_insertions_or_deletions = selected_columns
        .iter()
        .any(|column| column == "insertions" || column == "deletions");

    for commit_info in revwalk {
        let commit_info = commit_info.map_err(|err| err.to_string())?;
        let commit = repo
            .find_object(commit_info.id)
            .map_err(|err| err.to_string())?
            .into_commit();
        let commit_ref = commit.decode().map_err(|err| err.to_string())?;
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());

        for column_name in selected_columns {
            let column_name = column_name.as_str();
            let value = match column_name {
                "commit_id" => text_value(commit_info.id.to_string()),
                "name" => text_value(
                    commit_ref
                        .author()
                        .map(|author| author.name.to_string())
                        .unwrap_or_default(),
                ),
                "email" => text_value(
                    commit_ref
                        .author()
                        .map(|author| author.email.to_string())
                        .unwrap_or_default(),
                ),
                "datetime" => Box::new(DateTimeValue {
                    value: commit_info
                        .commit_time
                        .unwrap_or_else(|| commit_ref.time().map(|time| time.seconds).unwrap_or(0)),
                }),
                "insertions" | "deletions" | "files_changed" => {
                    let current = commit.tree().map_err(|err| err.to_string())?;
                    let previous = commit_info
                        .parent_ids()
                        .next()
                        .map(|id| {
                            repo.find_object(id)
                                .map_err(|err| err.to_string())
                                .and_then(|obj| {
                                    obj.into_commit().tree().map_err(|err| err.to_string())
                                })
                        })
                        .transpose()
                        .map_err(|err| err.to_string())?
                        .unwrap_or_else(|| repo.empty_tree());
                    rewrite_cache.clear_resource_cache();
                    diff_cache.clear_resource_cache();

                    let (mut insertions, mut deletions, mut files_changed) = (0, 0, 0);
                    previous
                        .changes()
                        .map_err(|err| err.to_string())?
                        .for_each_to_obtain_tree_with_cache(
                            &current,
                            &mut rewrite_cache,
                            |change| -> Result<_, Box<gix::object::blob::diff::init::Error>> {
                                files_changed += usize::from(change.entry_mode().is_no_tree());
                                if select_insertions_or_deletions {
                                    if let Ok(mut platform) = change.diff(&mut diff_cache) {
                                        if let Ok(Some(counts)) = platform.line_counts() {
                                            deletions += counts.removals;
                                            insertions += counts.insertions;
                                        }
                                    }
                                }
                                Ok(std::ops::ControlFlow::Continue(()))
                            },
                        )
                        .map_err(|err| err.to_string())?;

                    match column_name {
                        "insertions" => int_value(insertions as i64),
                        "deletions" => int_value(deletions as i64),
                        "files_changed" => int_value(files_changed as i64),
                        _ => null_value(),
                    }
                }
                _ => repo_metadata_value(column_name, &repo_path, &repo_name)
                    .unwrap_or_else(null_value),
            };
            values.push(value);
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}

fn select_tags(repo: &gix::Repository, selected_columns: &[String]) -> Result<Vec<Row>, String> {
    let platform = repo.references().map_err(|err| err.to_string())?;
    let tag_names = platform.tags().map_err(|err| err.to_string())?;
    let repo_path = repo_workdir_path(repo);
    let repo_name = repo_name_from_path(&repo_path);
    let mut rows: Vec<Row> = vec![];
    for tag_ref in tag_names.flatten() {
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());

        for column_name in selected_columns {
            let column_name = column_name.as_str();
            let value = match column_name {
                "name" => text_value(
                    tag_ref
                        .name()
                        .category_and_short_name()
                        .map_or_else(String::default, |(_, short_name)| short_name.to_string()),
                ),
                _ => repo_metadata_value(column_name, &repo_path, &repo_name)
                    .unwrap_or_else(null_value),
            };
            values.push(value);
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}
