pub fn version_string() -> String {
    format!(
        "{} {} {}",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_IGNORE_BUILD_DATE"),
        env!("GIT_IGNORE_GIT_HASH")
    )
}
