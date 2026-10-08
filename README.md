<p align="center">
  <img src="assets/gray-logo.svg" alt="gray" width="96">
</p>
<h1 align="center">gray-bash-env</h1>
<p align="center">Give toolchain commands a real login environment.</p>
<p align="center">
  <a href="https://github.com/vstaln/gray-bash-env/blob/main/LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-7aa2f7.svg">
  <img alt="rust" src="https://img.shields.io/badge/built%20with-rust-orange.svg">
</p>

Give toolchain commands a real login environment.

A sidecar plugin for [gray](https://github.com/vstaln/gray). It answers
`tool/before` on `bash` with `{"decision":"modify"}` prepending

```
[ -f ~/.profile ] && . ~/.profile; [ -f ~/.bashrc ] && . ~/.bashrc; 
```

—but only when it's likely to matter. The rewrite fires when:

- the command's **first token** names a watched tool (`node`, `npm`,
  `cargo`, `go`, `uv`, `pyenv` — matched by basename, so
  `~/.cargo/bin/cargo build` counts), and
- the command doesn't already `source`/`.` a file or run under
  `bash -l` / `--login`.

Everything else passes through untouched. Fail open: any problem → allow.

## State

- `~/.gray/bash-env/enabled` — `on`/`off` (default on)
- `~/.gray/bash-env/tools.txt` — watched tools, one per line, `#` comments;
  seeded with the defaults on first use

## Commands

- `/env on` / `/env off` — toggle
- `/env add <tool>` / `/env rm <tool>` — edit the watched list
- `/env list` — show the watched list

## Wire

`plugin/manifest`, `tool/before`, `command/run`, `plugin/shutdown`.
Protocol 2.0, hook `tool/before`. No capabilities required.

## Install

```sh
gray plugin install bash-env
```

## Develop

```sh
cargo test
gray account check      # entry point + manifest handshake
gray account publish    # check → build → release → publish to the gray registry
```

Bump `version` in `Cargo.toml` before each `publish`; the registry refuses to
republish a version.

---
Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
