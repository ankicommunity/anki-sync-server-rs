use config::{Environment, File};
use std::collections::HashMap;
pub fn addr() -> String {
    let envs = env_variables();
    let h = envs.get("host").unwrap();
    let p = envs.get("port").unwrap();
    format!("{}:{}", h, p)
}

/// return sync env vars in hashmap
pub fn env_variables() -> HashMap<String, String> {
    let mut settings = config::Config::default();
    settings
        // Add in `./Settings.toml`
        .merge(File::with_name("Settings"))
        .unwrap()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(Environment::with_prefix("host"))
        .unwrap()
        .merge(Environment::with_prefix("port"))
        .unwrap()
        .merge(Environment::with_prefix("data"))
        .unwrap()
        .merge(Environment::with_prefix("base"))
        .unwrap()
        .merge(Environment::with_prefix("auth"))
        .unwrap()
        .merge(Environment::with_prefix("session"))
        .unwrap()
        .merge(Environment::with_prefix("ssl"))
        .unwrap()
        .merge(Environment::with_prefix("cert"))
        .unwrap()
        .merge(Environment::with_prefix("key"))
        .unwrap()
        .merge(Environment::with_prefix("user"))
        .unwrap();

    settings.try_into::<HashMap<String, String>>().unwrap()
}

#[test]
fn test_env_vars() {
    println!("{:?}", env_variables())
}
