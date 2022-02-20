use std::env;
fn main() {
    let pat = "rustls";
    let key = format!("CARGO_FEATURE_{}", pat).to_uppercase();
    if env::var_os(key).is_some() {
        println!("{}", format!("cargo:rustc-cfg=feature=\"{}\"", pat))
    }
}
