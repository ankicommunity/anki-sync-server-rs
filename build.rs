use std::env;
fn main() {
    let pat = "rustls";
    let key = format!("CARGO_FEATURE_{}", pat).to_uppercase();
    match env::var_os(key) {
        Some(_) => {
            println!("{}", format!("cargo:rustc-cfg=feature=\"{}\"", pat))
        }
        None => {}
    }
}
