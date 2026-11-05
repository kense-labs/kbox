# Crossterm Examples

## 概述

本目录保存 crates/third-party/crossterm/examples 的说明和使用约束。

## 构建与验证

按所属工程的构建和测试合同使用本目录内容。

The examples are compatible with the latest release.  

## Structure

```
├── examples
│   └── interactive-test
│   └── event-*
│   └── stderr
```
| File Name                   | Description                    | Topics                                    |
|:----------------------------|:-------------------------------|:------------------------------------------|
| `examples/interactive-test` | interactive, walk through, demo | cursor, style, event                      |
| `event-*`                   | event reading demos            | (async) event reading                     |
| `stderr`                    | crossterm over stderr demo     | raw mode, alternate screen, custom output |
| `is_tty`                    | Is this instance a tty ?       | tty                                       |

## Run examples

```bash
$ cargo run --example [file name]
```

To run the interactive-demo go into the folder `examples/interactive-demo` and run `cargo run`.

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE) file for details.

[s2]: https://img.shields.io/badge/license-MIT-blue.svg
[l2]: LICENSE

[s5]: https://img.shields.io/discord/560857607196377088.svg?logo=discord
[l5]: https://discord.gg/K4nyTDB

[s7]: https://travis-ci.org/crossterm-rs/examples.svg?branch=master
