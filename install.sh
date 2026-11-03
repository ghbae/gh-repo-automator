#!/bin/sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' "エラー: Cargo が見つかりません。Rust をインストールしてから再実行してください。" >&2
    exit 1
fi

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
cargo install --path "$script_dir" --force

cargo_home=${CARGO_HOME:-"$HOME/.cargo"}
bin_dir="$cargo_home/bin"
binary="$bin_dir/gh-repo-automator"

if [ ! -x "$binary" ]; then
    printf 'エラー: インストール後の実行ファイルが見つかりません: %s\n' "$binary" >&2
    exit 1
fi

printf 'インストールしました: %s\n' "$binary"
case ":${PATH:-}:" in
    *":$bin_dir:"*) ;;
    *)
        printf 'PATH に未登録です。シェル設定に次の行を追加してください:\n'
        printf 'export PATH="%s:$PATH"\n' "$bin_dir"
        ;;
esac

printf '\nTAB補完を有効にするには、シェル設定に次の行を追加してください:\n'
printf '  bash (~/.bashrc): source <(gh-repo-automator --completion bash)\n'
printf '  zsh  (~/.zshrc):  source <(gh-repo-automator --completion zsh)\n'
