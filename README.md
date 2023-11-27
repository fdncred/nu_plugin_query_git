# nu_plugin_query_git

This nushell plugin will allow you to query git via [gitql](https://github.com/AmrDeveloper/GQL).

## Usage:

Just to get a feel, here's some debug output.

```
❯ query git "SELECT * FROM commits limit 5" | update message {|r| $r.message | str substring 0..25}

Analysis:
Frontend : 110.083µs
Engine   : 153.207375ms
Total    : 153.317458ms


╭─#─┬────────message────────┬───────name────────┬─────────title─────────┬───────commit_id───────┬─────────email─────────┬───────datetime────────┬──────────repo──────────╮
│ 0 │ Cp target expansion   │ Artemiy           │ Cp target expansion   │ 1ff8c2d81dea68e90a6a6 │ artem-itf@yandex.ru   │ 2023-11-25            │ /Users/fdncred/src/nus │
│   │ (#111                 │                   │ (#11152)              │ 1b68a50a2098b1d6a8a   │                       │ 15:42:20.000          │ hell/.git/             │
│ 1 │ add shape             │ Darren Schroeder  │ add shape             │ d77f1753c2714d9739628 │ 343840+fdncred@users. │ 2023-11-25            │ /Users/fdncred/src/nus │
│   │ `ExternalResolv       │                   │ `ExternalResolved` to │ 6e0e94e2d996e75b343   │ noreply.github.com    │ 15:42:05.000          │ hell/.git/             │
│   │                       │                   │  show found externals │                       │                       │                       │                        │
│   │                       │                   │  via syntax           │                       │                       │                       │                        │
│   │                       │                   │ highlighting in the   │                       │                       │                       │                        │
│   │                       │                   │ repl (#11135)         │                       │                       │                       │                        │
│ 2 │ fix the link to the   │ Antoine Stevan    │ fix the link to the   │ 85c6047b71a15464e248c │ 44101798+amtoine@user │ 2023-11-24            │ /Users/fdncred/src/nus │
│   │ `nu_s                 │                   │ `nu_scripts` in `std  │ 7b9cdab5e668468489e   │ s.noreply.github.com  │ 18:03:07.000          │ hell/.git/             │
│   │                       │                   │ clip` deprecation     │                       │                       │                       │                        │
│   │                       │                   │ (#11150)              │                       │                       │                       │                        │
│ 3 │ Add more descriptive  │ Marika Chlebowska │ Add more descriptive  │ d37893cca0aac573e623c │ marika.c@protonmail.c │ 2023-11-24            │ /Users/fdncred/src/nus │
│   │ erro                  │                   │ error message when    │ a0e25a5e7ed1828b02f   │ om                    │ 13:45:01.000          │ hell/.git/             │
│   │                       │                   │ passing list to       │                       │                       │                       │                        │
│   │                       │                   │ from_csv (#10962)     │                       │                       │                       │                        │
│ 4 │ Fix release and       │ Justin Ma         │ Fix release and       │ 95ac436d2652120d1b59a │ hustcer@outlook.com   │ 2023-11-24            │ /Users/fdncred/src/nus │
│   │ nightly b             │                   │ nightly build         │ f0b020bf18deaedaec8   │                       │ 01:10:39.000          │ hell/.git/             │
│   │                       │                   │ workflow (#11146)     │                       │                       │                       │                        │
╰─#─┴────────message────────┴───────name────────┴─────────title─────────┴───────commit_id───────┴─────────email─────────┴───────datetime────────┴──────────repo──────────╯
```

