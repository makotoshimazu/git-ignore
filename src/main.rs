fn main() {
    if let Err(error) = git_ignore::run_from_env() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
