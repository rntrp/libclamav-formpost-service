use std::path::PathBuf;

fn main() {
    let libclamav = pkg_config::Config::new()
        .atleast_version("1.4.0")
        .probe("libclamav")
        .unwrap();
    let mut include_paths = libclamav.include_paths.clone();
    if let Some(val) = std::env::var_os("OPENSSL_ROOT_DIR") {
        let mut openssl_include_dir = PathBuf::from(val);
        openssl_include_dir.push("include");
        include_paths.push(openssl_include_dir);
    }
}
