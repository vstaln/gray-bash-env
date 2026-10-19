#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/gray-bash-env.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/gray-bash-env" ;;
    *) rel="x86_64-apple-darwin/gray-bash-env" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/gray-bash-env" ;;
    *) rel="x86_64-unknown-linux-musl/gray-bash-env" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/gray-bash-env.exe" "$here/target/release/gray-bash-env" "$here/target/debug/gray-bash-env.exe" "$here/target/debug/gray-bash-env"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "gray-bash-env binary not found in $here" >&2
exit 1
