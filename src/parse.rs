use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg, ArgMatches};

pub mod conf {
    use std::fs;
    use std::path::Path;

    pub fn write_conf(p: &Path) {
        let content = r#"
[address]
host="0.0.0.0"
port = "27701"
#set real data path with ENV ANKISYNCD_ROOT,if not exist,
# use current executable path ,only set filename
[path]
data_root = "collections"
auth_db_path = "auth.db"
session_db_path = "session.db"
        
# following fields will be added 
#into auth.db if not empty,and two fields must not be empty
[account]
username=""
userpassword=""
        
# embeded encrypted http /https credential if in Intranet
# true to enable ssl or false
[localcert]
ssl_enable="false"
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
