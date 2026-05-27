# git-ignore

`git-ignore` は [github/gitignore](https://github.com/github/gitignore) のルート直下テンプレートを一覧表示し、指定したテンプレートをカレントディレクトリの `.gitignore` に追記するCLIです。

## Install

```sh
brew install makotoshimazu/tap/git-ignore
```

## Usage

```sh
git-ignore --version
git-ignore
git-ignore list
git-ignore Node
git-ignore --refresh-cache list
git-ignore cache clear
git-ignore completion zsh
git-ignore completion bash
```

`git-ignore` をPATHに置くと、Gitの外部サブコマンドとして `git ignore Node` でも実行できます。

```sh
git ignore
git ignore Node
```

`git-ignore Node` は `https://raw.githubusercontent.com/github/gitignore/refs/heads/main/Node.gitignore` を取得し、cwdの `.gitignore` 末尾に追記します。既存エントリの重複除去はしません。

## Cache

テンプレート一覧と取得済みテンプレートはデフォルトで24時間キャッシュします。

- manifest: `~/.cache/git-ignore/manifest.json`
- templates: `~/.cache/git-ignore/templates/{Name}.gitignore`
- config: `~/.config/git-ignore/config.toml`

設定ファイル:

```toml
[cache]
enabled = true
ttl_seconds = 86400
```

`--refresh-cache` は設定ファイルより優先され、その実行ではネットワークから取得し直してローカルキャッシュを更新します。

## Completion

Homebrewでインストールした場合、zsh completionはHomebrewの `share/zsh/site-functions` に自動で配置されます。

自分で足す場合は、以下のコマンドを使ってください。

```sh
git-ignore completion zsh > ~/.zsh/completions/_git-ignore
git-ignore completion bash > ~/.bash_completion.d/git-ignore
```

補完スクリプトは内部で `git-ignore __complete <prefix>` を呼びます。候補はgithub/gitignoreのルート直下 `{Name}.gitignore` から生成されます。

## Development

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

リリース前の確認:

```sh
goreleaser check
goreleaser release --snapshot --clean --skip=publish
```

## License

Apache-2.0
