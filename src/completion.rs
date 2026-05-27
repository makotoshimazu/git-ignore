use anyhow::{Result, bail};

pub(crate) fn completion_script(shell: &str) -> Result<&'static str> {
    match shell {
        "bash" => Ok(BASH),
        "zsh" => Ok(ZSH),
        other => bail!("unsupported shell '{other}'; expected bash or zsh"),
    }
}

const BASH: &str = r#"# bash completion for git-ignore
_git_ignore_complete() {
  local cur
  cur="${COMP_WORDS[COMP_CWORD]}"
  mapfile -t COMPREPLY < <(git-ignore __complete "${cur}" 2>/dev/null)
}

complete -F _git_ignore_complete git-ignore

# Git's bash completion looks for _git_<subcommand> helpers for external commands.
_git_ignore() {
  _git_ignore_complete
}
"#;

const ZSH: &str = r#"#compdef git-ignore git-ignore

_git_ignore() {
  local -a candidates
  candidates=("${(@f)$(git-ignore __complete "${words[CURRENT]}" 2>/dev/null)}")
  _describe 'gitignore templates' candidates
}

compdef _git_ignore git-ignore

# Git's zsh completion can look for _git-<subcommand> helpers for external commands.
_git-ignore() {
  _git_ignore "$@"
}
"#;
