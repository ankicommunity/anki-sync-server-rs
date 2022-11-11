use std::env;
use std::path::Path;

fn main() {
    // should consider native build on arm platform

    // used in cross compile while building with CD
    // such as arm-unknown-linux-musleabihf
    let target = env::var("TARGET").expect("TARGET was not set");
    if target.contains("arm") && target.contains("musl") {
        // find and link static sqlite3 lib
        let sql = Path::new(&env::current_dir().unwrap()).join("sql/lib");
        println!("cargo:rustc-link-search=native={}", sql.display());
        println!("cargo:rustc-link-lib=static=sqlite3");
    }
    if target.contains("aarch64") && target.contains("musl") {
        // find and link static sqlite3 lib
        let sql = Path::new(&env::current_dir().unwrap()).join("sql/lib");
        println!("cargo:rustc-link-search=native={}", sql.display());
        println!("cargo:rustc-link-lib=static=sqlite3");
    }
    let pat = "tls";
    let key = format!("CARGO_FEATURE_{}", pat).to_uppercase();
    if env::var_os(key).is_some() {
        println!("cargo:rustc-cfg=feature=\"{}\"", pat)
    }
}
