# git-ignore

`git-ignore` は [github/gitignore](https://github.com/github/gitignore) のルート直下テンプレートを一覧表示し、指定したテンプレートをカレントディレクトリの `.gitignore` に追記するCLIです。

## Install

Homebrew配布はGitHub ReleasesとGoReleaserで行う想定です。

```sh
brew tap makotoshimazu/tap
brew install git-ignore
```

## Usage

```sh
git-ignore --version
git-ignore
git-ignore list
git-ignore Node
git-ignore --no-cache list
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

テンプレート一覧と取得済みテンプレートはデフォルトで10分間キャッシュします。

- manifest: `~/.cache/git-ignore/manifest.json`
- templates: `~/.cache/git-ignore/templates/{Name}.gitignore`
- config: `~/.config/git-ignore/config.toml`

設定ファイル:

```toml
[cache]
enabled = true
ttl_seconds = 600
```

`--no-cache` は設定ファイルより優先され、その実行ではキャッシュを読まず、書き込みもしません。

## Completion

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

## Release

GoReleaserでmacOS/Linux向けのx86_64/aarch64バイナリとHomebrew tap更新を作成します。npm配布はv1後続タスクです。

## License

Apache-2.0
