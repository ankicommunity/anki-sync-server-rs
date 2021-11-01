use crate::db::fetchone;
use crate::envconfig::env_variables;
use anki::sync::http::HostKeyRequest;
use rand::{rngs::OsRng, RngCore};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
fn path_exists(path: &str) -> io::Result<bool> {
    Ok(Path::new(path).exists())
}
fn create_salt() -> String {
    // create salt
    let mut key = [0u8; 8];
    OsRng.fill_bytes(&mut key);
    let salt = hex::encode(key);
    salt
}
fn set_password_for_user(username: &str, new_password: &str) -> rusqlite::Result<()> {
    if user_exists(username).unwrap() {
        let salt = create_salt();
        let hash = create_pass_hash(username, new_password, &salt);
        let sql = "UPDATE auth SET hash=? WHERE username=?";
        let conn = Connection::open("auth.db").unwrap();
        conn.execute(sql, [hash.as_str(), username]).unwrap();
        conn.close().unwrap();
    }

    Ok(())
}

fn create_user_dir(username: &str) -> io::Result<()> {
    let path = Path::new("collections").join(username);
    if !path.exists() {
        fs::create_dir_all(path).unwrap();
    }

    Ok(())
}
fn add_user_to_auth_db(username: &str, password: &str) -> io::Result<()> {
    let salt = create_salt();
    let pass_hash = create_pass_hash(username, password, &salt);
    let sql = "INSERT INTO auth VALUES (?, ?)";
    let conn = Connection::open("auth.db").unwrap();
    conn.execute(sql, [username, pass_hash.as_str()]).unwrap();
    conn.close().unwrap();
    create_user_dir(username).unwrap();
    Ok(())
}
pub fn add_user(args: &[String]) -> io::Result<()> {
    let username = &args[0];
    let password = &args[1];
    add_user_to_auth_db(username, password).unwrap();
    Ok(())
}
pub fn passwd(args: &[String]) -> io::Result<()> {
    let username = &args[0];
    let password = &args[1];
    set_password_for_user(username, &password).unwrap();
    Ok(())
}
pub fn del_user(username: &str) -> io::Result<()> {
    let sql = "DELETE FROM auth WHERE username=?";
    let conn = Connection::open("auth.db").unwrap();
    conn.execute(sql, [username]).unwrap();
    conn.close().unwrap();

    Ok(())
}

pub fn create_auth_db() -> io::Result<()> {
    let sql = "CREATE TABLE IF NOT EXISTS auth
(username VARCHAR PRIMARY KEY, hash VARCHAR)";
    let conn = Connection::open("auth.db").unwrap();
    conn.execute(sql, []).unwrap();
    conn.close().unwrap();

    Ok(())
}
/// into a vec
fn read_args_from_cmd() -> Vec<String> {
    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    let args = s.trim()
        .split_ascii_whitespace()
        .into_iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>();
    args
}
/// command-line user management,ie add user
pub fn user_manage() {
    println!("1) add user");
    println!("2) delete user");
    println!("3) change user password");
    println!("4) show existing users");
    print!("your choice?");
    io::stdout().flush().unwrap();
    let mut out = String::new();
    io::stdin().read_line(&mut out).unwrap();
    match out.trim() {
        "1" => {
            // add user
            // notice if user already exists
            println!("input username and password,separate by whitespace, ie: user pass");
            print!("your input?");
            io::stdout().flush().unwrap();
            let args = read_args_from_cmd();
            if args.len() == 2 {
                add_user(&args).unwrap();
            } else {
                panic!("error input format")
            }
        }
        "2" => {
            // delete user
            println!("input to-be-deleted user ,multi users available,separated by whitespace,ie: user1 user2");
            print!("your input?");
            io::stdout().flush().unwrap();
            let args = read_args_from_cmd();
            for u in args {
                del_user(&u).unwrap();
            }
        }
        "3" => {
            // change user password
            println!("input username and to-be-changed password,separate by whitespace, ie: user newpass");
            print!("your input?");
            io::stdout().flush().unwrap();
            let args = read_args_from_cmd();
            if args.len() == 2 {
                passwd(&args).unwrap();
            } else {
                panic!("error input len")
            }
        }
        "4" => {
            // show existing users
            println!("existing users are as follows");
            let user_list = user_list().unwrap();
            if let Some(v) = user_list {
                for i in v {
                    println!("{}", i)
                }
            } else {
                println!()
            }
        }
        _ => {}
    }
}
pub fn user_list() -> io::Result<Option<Vec<String>>> {
    let sql = "SELECT username FROM auth";
    let conn = Connection::open("auth.db").unwrap();
    let mut stmt = conn.prepare(sql).unwrap();
    let rows = stmt.query_map([], |r| r.get(0)).unwrap();

    let v = rows
        .into_iter()
        .map(|r| r.unwrap())
        .collect::<Vec<String>>();
    let r = if v.is_empty() { None } else { Some(v) };
    Ok(r)
}
fn user_exists(username: &str) -> io::Result<bool> {
    let uservec = user_list().unwrap();
    match uservec {
        Some(x) if x.contains(&username.to_string()) => Ok(true),
        _ => {
            println!("User {} doesn't exist", username);
            Ok(false)
        }
    }
}
fn create_pass_hash(username: &str, password: &str, salt: &str) -> String {
    // create a Sha256 object
    let mut hasher = Sha256::new();
    // write input message
    hasher.update(username);
    hasher.update(password);
    hasher.update(&salt);
    // read hash digest and consume hasher
    let result = hasher.finalize();
    let pass_hash = format!("{:x}{}", result, salt);
    pass_hash
}

pub fn authenticate(hkreq: &HostKeyRequest) -> bool {
    let auth_db = &env_variables()["auth_db_path"];
    let conn = Connection::open(auth_db).unwrap();
    let sql = "SELECT hash FROM auth WHERE username=?";
    let db_hash: Option<String> = fetchone(&conn, sql, Some(&hkreq.username)).unwrap();
    conn.close().unwrap();
    if let Some(expect_value) = db_hash {
        let salt = &expect_value[(&expect_value.chars().count() - 16)..];
        let actual_value = create_pass_hash(&hkreq.username, &hkreq.password, salt);
        if actual_value == expect_value {
            println!("Authentication succeeded for  user {}.", &hkreq.username);
            return true;
        } else {
            println!("Authentication failed for user {}", &hkreq.username);
            return false;
        }
    } else {
        println!(
            "Authentication failed for nonexistent user {}.",
            &hkreq.username
        );
        return false;
    }
}

#[test]
fn test_relpath() {
    let r = "src";
    println!("{}", env::current_dir().unwrap().display());
    //    [Ok(DirEntry("src\\appconfig.rs")),]
    println!("{:?}", Path::new(r).read_dir().unwrap().collect::<Vec<_>>())
}
//extract salt
// #[test]
// fn test_crt_pass_hash(){

//   let h=  create_pass_hash("1","2");
// let s=&h[(h.chars().count()-16)..];
// println!("{}",s);
// }
