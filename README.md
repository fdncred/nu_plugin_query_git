# nu_plugin_query_git

This nushell plugin will allow you to query git via [gitql](https://github.com/AmrDeveloper/GQL).

## Usage:

Just to get a feel, here's some debug output.

### Show the tables available to be queried
```nushell
❯ query git 'show tables' 
╭─#─┬──table───╮
│ 0 │ branches │
│ 1 │ commits  │
│ 2 │ diffs    │
│ 3 │ refs     │
│ 4 │ tags     │
╰─#─┴──table───╯
```
###  Show the first 10 refs
```nushell
❯ query git 'select * from refs limit 10' 
╭─#─┬────name─────┬────────full_name─────────┬──type──┬──────────────────repo──────────────────┬──────repo_name──────╮
│ 0 │ main        │ refs/heads/main          │ branch │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 1 │ origin/HEAD │ refs/remotes/origin/HEAD │ remote │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 2 │ origin/main │ refs/remotes/origin/main │ remote │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 3 │ v0.10.0     │ refs/tags/v0.10.0        │ tag    │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 4 │ v0.11.0     │ refs/tags/v0.11.0        │ tag    │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 5 │ v0.12.0     │ refs/tags/v0.12.0        │ tag    │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 6 │ v0.13.0     │ refs/tags/v0.13.0        │ tag    │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 7 │ v0.14.0     │ refs/tags/v0.14.0        │ tag    │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 8 │ v0.15.0     │ refs/tags/v0.15.0        │ tag    │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
│ 9 │ v0.16.0     │ refs/tags/v0.16.0        │ tag    │ /Users/fdncred/src/nu_plugin_query_git │ nu_plugin_query_git │
╰─#─┴────name─────┴────────full_name─────────┴──type──┴──────────────────repo──────────────────┴──────repo_name──────
```
### Show the commits schema as YAML (JSON, YAML, CSV)
```nushell
❯ query git 'describe commits' --output yaml 
- field: commit_id
  type: Text
- field: title
  type: Text
- field: message
  type: Text
- field: author_name
  type: Text
- field: author_email
  type: Text
- field: committer_name
  type: Text
- field: committer_email
  type: Text
- field: datetime
  type: Date
- field: parents_count
  type: Int
- field: repo
  type: Text
- field: repo_name
  type: Text
```
### Query commits from the current repo and return CSV (JSON, YAML, CSV)
```nushell
❯ query git 'select title, datetime from commits' --repo . --output csv
title,datetime
update to nushell 0.112.2,2026-04-20 14:40:11.000
update to nushell 0.111.0,2026-03-02 21:29:19.000
update to nushell 0.110.0,2026-01-20 14:32:18.000
update version,2025-11-30 19:43:00.000
update to nushell 0.109.0,2025-11-30 19:28:38.000
update to nushell 0.108.0,2025-10-15 15:28:52.000
update to nushell 0.107.0,2025-09-06 23:51:50.000
update to nushell 0.106.0,2025-07-24 14:39:24.000
update to nushell 0.105.1 and 2024,2025-06-17 15:38:49.000
update to nushell 0.104,2025-04-30 21:50:16.000
update to nushell 0.103.0,2025-03-29 19:04:11.000
update to nushell 0.102.0,2025-02-10 15:56:53.000
update to nushell 0.101.0,2024-12-25 12:05:29.000
update to nushell 0.100.0,2024-11-14 17:53:10.000
update to nushell 0.99.1,2024-10-21 20:54:44.000
update to nushell 0.98.0,2024-09-18 14:28:49.000
Merge pull request #2 from fdncred/0972_dev,2024-09-09 16:06:31.000
Merge pull request #1 from fdncred/0971_publishing,2024-09-09 16:04:51.000
update to nushell 0.97.2 for dev,2024-09-09 16:06:03.000
update to nushell 0.97.2,2024-08-22 13:01:08.000
...
```
### Query multiple repositories using a Nushell list
```nushell
❯ query git 'show tables' --repos [.] 
╭─#─┬──table───╮
│ 0 │ branches │
│ 1 │ commits  │
│ 2 │ diffs    │
│ 3 │ refs     │
│ 4 │ tags     │
╰─#─┴──table───╯
```
### Limit output to the first 20 rows of results
```nushell
❯ query git 'select * from refs' --pagination --page-size
╭─#──┬───────────────────────name────────────────────────┬───────────────────────────full_name────────────────────────────┬──type──┬────────────repo────────────┬─repo_name─╮
│  0 │ bump_idx_deps_081                                 │ refs/heads/bump_idx_deps_081                                   │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  1 │ dc-glob-integration                               │ refs/heads/dc-glob-integration                                 │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  2 │ dynamic_experimental_options                      │ refs/heads/dynamic_experimental_options                        │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  3 │ fix_hide_env_20260506                             │ refs/heads/fix_hide_env_20260506                               │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  4 │ help_update_work_nested                           │ refs/heads/help_update_work_nested                             │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  5 │ improve_escaping                                  │ refs/heads/improve_escaping                                    │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  6 │ improve_repl_err_handling                         │ refs/heads/improve_repl_err_handling                           │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  7 │ main                                              │ refs/heads/main                                                │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  8 │ pr/fdncred/18170                                  │ refs/heads/pr/fdncred/18170                                    │ branch │ /Users/fdncred/src/nushell │ nushell   │
│  9 │ separate_nuver_nuprotover                         │ refs/heads/separate_nuver_nuprotover                           │ branch │ /Users/fdncred/src/nushell │ nushell   │
│ 10 │ skim_cmd                                          │ refs/heads/skim_cmd                                            │ branch │ /Users/fdncred/src/nushell │ nushell   │
│ 11 │ table_hints                                       │ refs/heads/table_hints                                         │ branch │ /Users/fdncred/src/nushell │ nushell   │
│ 12 │ work_on_table_heuristic                           │ refs/heads/work_on_table_heuristic                             │ branch │ /Users/fdncred/src/nushell │ nushell   │
│ 13 │ Dexterity104/HEAD                                 │ refs/remotes/Dexterity104/HEAD                                 │ remote │ /Users/fdncred/src/nushell │ nushell   │
│ 14 │ Dexterity104/fix/ls-mode-acl-indicator            │ refs/remotes/Dexterity104/fix/ls-mode-acl-indicator            │ remote │ /Users/fdncred/src/nushell │ nushell   │
│ 15 │ Dexterity104/fix/parse-char-lbrace-before-capture │ refs/remotes/Dexterity104/fix/parse-char-lbrace-before-capture │ remote │ /Users/fdncred/src/nushell │ nushell   │
│ 16 │ Dexterity104/main                                 │ refs/remotes/Dexterity104/main                                 │ remote │ /Users/fdncred/src/nushell │ nushell   │
│ 17 │ Juhan280/HEAD                                     │ refs/remotes/Juhan280/HEAD                                     │ remote │ /Users/fdncred/src/nushell │ nushell   │
│ 18 │ Juhan280/execute_host_command-closure             │ refs/remotes/Juhan280/execute_host_command-closure             │ remote │ /Users/fdncred/src/nushell │ nushell   │
│ 19 │ Juhan280/main                                     │ refs/remotes/Juhan280/main                                     │ remote │ /Users/fdncred/src/nushell │ nushell   │
╰─#──┴───────────────────────name────────────────────────┴───────────────────────────full_name────────────────────────────┴──type──┴────────────repo────────────┴─repo_name─╯
```
### Run a query and print analysis timing information
```nushell
❯ query git 'select count(*) from commits' --analysis 


Analysis:
Frontend : 169.625µs
Engine   : 8.026417ms
Total    : 8.196042ms


╭─#─┬─column_0─╮
│ 0 │       40 │
╰───┴──────────╯
```
### Show title and datetime of commits with conventional title 'feat'
```nushell
❯ query git 'SELECT title, datetime FROM commits WHERE commit_conventional(title) = "feat"'
╭─#──┬───────────────────────────────────────────────────title────────────────────────────────────────────────────┬───datetime────╮
│  0 │ feat: accept optional column argument for `grid` command (#18187)                                          │ 5 days ago    │
│  1 │ feat: add `from md` command to convert markdown text into structured data (#17937)                         │ 2 months ago  │
│  2 │ feat: add `config.auto_cd_always` option (#17651)                                                          │ 2 months ago  │
│  3 │ feat: enhance script argument parsing to handle newlines and whitespace correctly (#17826)                 │ 2 months ago  │
│  4 │ feat: `group-by` flag to delete column after grouping by it (#17787)                                       │ 2 months ago  │
│  5 │ feat: `test_record!` macro for convenience (#17797)                                                        │ 2 months ago  │
│  6 │ feat: make `Span`'s `Debug` impl succinct (#17782)                                                         │ 2 months ago  │
│  7 │ feat: add ToStart/ToEnd reedline events (#17747)                                                           │ 2 months ago  │
│  8 │ feat: add support for custom history file path in config (#17425)                                          │ 2 months ago  │
│  9 │ feat: add `str escape-regex` (#17703)                                                                      │ 2 months ago  │
│ 10 │ feat: add `path_columns` to PipelineMetadata for flexible path rendering (#17540)                          │ 3 months ago  │
│ 11 │ feat: add linewise and non-blank start edit commands (#17508)                                              │ 3 months ago  │
│ 12 │ feat: enable OSC133 click events via reedline (#17491)                                                     │ 3 months ago  │
│ 13 │ feat: add closure parameter to `metadata set` (#16976)                                                     │ 6 months ago  │
...
```

