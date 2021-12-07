use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg, ArgMatches};

pub mod conf {
    use config::{Config, ConfigError, File};
    use serde::Deserialize;
    use std::fs;
    use std::path::Path;
    #[derive(Debug, Deserialize)]
    pub struct Address {
        pub host: String,
        pub port: String,
    }
    #[derive(Debug, Deserialize)]
    pub struct Paths {
        pub root_dir: String,
        pub data_root: String,
        pub auth_db_path: String,
        pub session_db_path: String,
    }
    #[derive(Debug, Deserialize)]
    pub struct Account {
        pub username: String,
        pub password: String,
    }
    #[derive(Debug, Deserialize)]
    pub struct LocalCert {
        pub ssl_enable: bool,
        pub cert_file: String,
        pub key_file: String,
    }
    #[derive(Debug, Deserialize)]
    pub struct Settings {
        pub address: Address,
        pub paths: Paths,
        pub account: Account,
        pub localcert: LocalCert,
    }
    impl Settings {
        // alaways read config file from the same dir as executable
        pub fn new() -> Result<Self, ConfigError> {
            let mut s = Config::default();

            // Start off by merging in the "default" configuration file
            s.merge(File::with_name("Settings"))?;

            let root = s.get_str("paths.root_dir").unwrap();
            s.set(
                "paths.data_root",
                format!("{}", Path::new(&root).join("collections").display()),
            )?;
            s.set(
                "paths.auth_db_path",
                format!("{}", Path::new(&root).join("auth.db").display()),
            )?;
            s.set(
                "paths.session_db_path",
                format!("{}", Path::new(&root).join("session.db").display()),
            )?;

            s.try_into()
        }
    }
    /// create configure file and write contents to it
    pub fn create_conf(p: &Path) {
        let content = r#"
[address]
host="0.0.0.0"
port = "27701"
# use current executable path ,only set filename
[paths]
# set root_dir as working dir where server data(collections folder) and database(auth.db...) reside
root_dir="."
#following three lines are unnessesary and can be skipped
 data_root = ""
 auth_db_path = ""
 session_db_path = ""
        
# user will be added 
#into auth.db if not empty,and two fields must not be empty
[account]
username=""
password=""
        
# embeded encrypted http /https credential if in Intranet
# true to enable ssl or false
[localcert]
ssl_enable=false
cert_file=""
key_file=""
"#;
        if !p.exists() {
            fs::write(&p, content).unwrap();
        }
    }
}
/// construct a argument parser
pub fn parse() -> ArgMatches {
    App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .about("Sets a custom config file,ie -c ankisyncd.toml")
                .default_value("Settings.toml")
                .takes_value(true),
        )
        .subcommand(
            App::new("user")
                .short_flag('U')
                .about("user management,interact with db CRUD actions")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::new("add")
                        .long("add")
                        .short('a')
                        .about("create user account, i.e.-a user password")
                        .value_names(&["username", "password"])
                        .takes_value(true)
                        .multiple_values(true)
                        .number_of_values(2),
                )
                .arg(
                    Arg::new("del")
                        .long("del")
                        .short('d')
                        .about("delete users,allow for multi-users, i.e.-d  user1 user2")
                        .value_name("username")
                        .takes_value(true)
                        .multiple_values(true)
                        .min_values(1),
                )
                .arg(
                    Arg::new("pass")
                        .long("pass")
                        .short('p')
                        .about("change user's password, i.e.-p user newpassword")
                        .value_names(&["username", "newpassword"])
                        .takes_value(true)
                        .multiple_values(true)
                        .number_of_values(2),
                )
                .arg(
                    Arg::new("list")
                        .about("list all usernames extracted from db ,i.e. -l")
                        .long("list")
                        .short('l'),
                ),
        )
        .get_matches()
}
