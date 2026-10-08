# gray-bash-env

Give toolchain commands a real login environment.

A sidecar plugin for [gray](https://github.com/vstaln/gray). Port of pi's
`bash-spawn-hook.ts` (MIT): where the original rebuilt the bash tool with a
`source ~/.profile` spawnHook, this sidecar answers `tool/before` on `bash`
with `{"decision":"modify"}` prepending

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
