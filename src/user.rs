use crate::db::fetchone;
use crate::parse::conf::Account;
use anki::sync::http::HostKeyRequest;
use clap::ArgMatches;
use rand::{rngs::OsRng, RngCore};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
#[allow(unused_imports)]
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
fn create_salt() -> String {
    // create salt
    let mut key = [0u8; 8];
    OsRng.fill_bytes(&mut key);
    hex::encode(key)
}
fn set_password_for_user<P: AsRef<Path>>(
    username: &str,
    new_password: &str,
    dbpath: P,
) -> rusqlite::Result<()> {
    if user_exists(username, &dbpath).unwrap() {
        let salt = create_salt();
        let hash = create_pass_hash(username, new_password, &salt);
        let sql = "UPDATE auth SET hash=? WHERE username=?";
        let conn = Connection::open(dbpath).unwrap();
        conn.execute(sql, [hash.as_str(), username]).unwrap();
        conn.close().unwrap();
    }

    Ok(())
}

fn create_user_dir(path: PathBuf) -> io::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).unwrap();
    }

    Ok(())
}
fn add_user_to_auth_db<P: AsRef<Path>>(
    username: &str,
    password: &str,
    dbpath: P,
) -> io::Result<()> {
    let salt = create_salt();
    let pass_hash = create_pass_hash(username, password, &salt);
    let sql = "INSERT INTO auth VALUES (?, ?)";
    let conn = Connection::open(&dbpath).unwrap();
    conn.execute(sql, [username, pass_hash.as_str()]).unwrap();
    conn.close().unwrap();
    let user_path = dbpath.as_ref().to_owned().parent().unwrap().join(username);
    create_user_dir(user_path).unwrap();
    Ok(())
}
pub fn add_user<P: AsRef<Path>>(args: &[String], dbpath: P) -> io::Result<()> {
    let username = &args[0];
    let password = &args[1];
    add_user_to_auth_db(username, password, dbpath).unwrap();
    Ok(())
}
fn passwd<P: AsRef<Path>>(args: &[String], dbpath: P) -> io::Result<()> {
    let username = &args[0];
    let password = &args[1];
    set_password_for_user(username, password, dbpath).unwrap();
    Ok(())
}
fn del_user<P: AsRef<Path>>(username: &str, dbpath: P) -> io::Result<()> {
    let sql = "DELETE FROM auth WHERE username=?";
    let conn = Connection::open(dbpath).unwrap();
    conn.execute(sql, [username]).unwrap();
    conn.close().unwrap();

    Ok(())
}
// insert record into db if username is not empty in Settings.toml
pub fn create_account<P: AsRef<Path>>(account: Account, dbpath: P) {
    // insert record into db if username is not empty,
    let name = account.username;
    let pass = account.password;
    // insert record into db if user if not empty,
    // else start server
    if !name.is_empty() {
        if !pass.is_empty() {
            // look up in db to check if user exist
            let user_list = user_list(&dbpath).unwrap();
            //  insert into db if username is not included indb query result
            if let Some(v) = user_list {
                if !v.contains(&name) {
                    add_user(&[name, pass], &dbpath).unwrap();
                }
            } else {
                add_user(&[name, pass], &dbpath).unwrap();
            }
        } else {
            panic!("user fields are not allowed for empty")
        }
    }
}
pub fn create_auth_db<P: AsRef<Path>>(p: P) -> io::Result<()> {
    let sql = "CREATE TABLE IF NOT EXISTS auth
(username VARCHAR PRIMARY KEY, hash VARCHAR)";
    let conn = Connection::open(p).unwrap();
    conn.execute(sql, []).unwrap();
    conn.close().unwrap();

    Ok(())
}

/// command-line user management
pub fn user_manage<P: AsRef<Path>>(matches: ArgMatches, dbpath: P) {
    match matches.subcommand() {
        Some(("user", user_mach)) => {
            if user_mach.is_present("add") {
                let acnt = user_mach
                    .values_of("add")
                    .unwrap()
                    .into_iter()
                    .map(|a| a.to_owned())
                    .collect::<Vec<_>>();
                add_user(&acnt, &dbpath).unwrap();
            }
            if user_mach.is_present("del") {
                let users = user_mach
                    .values_of("del")
                    .unwrap()
                    .into_iter()
                    .map(|a| a.to_owned())
                    .collect::<Vec<_>>();
                for u in users {
                    del_user(&u, &dbpath).unwrap();
                }
            }
            if user_mach.is_present("list") {
                let user_list = user_list(&dbpath).unwrap();
                if let Some(v) = user_list {
                    for i in v {
                        println!("{}", i)
                    }
                } else {
                    println!()
                }
            }
            if user_mach.is_present("pass") {
                let acnt = user_mach
                    .values_of("pass")
                    .unwrap()
                    .into_iter()
                    .map(|a| a.to_owned())
                    .collect::<Vec<_>>();
                passwd(&acnt, &dbpath).unwrap();
            }
        }

        _ => unreachable!(),
    }
}
pub fn user_list<P: AsRef<Path>>(dbpath: P) -> io::Result<Option<Vec<String>>> {
    let sql = "SELECT username FROM auth";
    let conn = Connection::open(dbpath).unwrap();
    let mut stmt = conn.prepare(sql).unwrap();
    let rows = stmt.query_map([], |r| r.get(0)).unwrap();

    let v = rows
        .into_iter()
        .map(|r| r.unwrap())
        .collect::<Vec<String>>();
    let r = if v.is_empty() { None } else { Some(v) };
    Ok(r)
}
fn user_exists<P: AsRef<Path>>(username: &str, dbpath: P) -> io::Result<bool> {
    let uservec = user_list(dbpath).unwrap();
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

pub fn authenticate<P: AsRef<Path>>(hkreq: &HostKeyRequest, auth_db_path: P) -> bool {
    let auth_db = auth_db_path.as_ref();

    let conn = Connection::open(auth_db).unwrap();
    let sql = "SELECT hash FROM auth WHERE username=?";
    let db_hash: Option<String> = fetchone(&conn, sql, Some(&hkreq.username)).unwrap();
    conn.close().unwrap();
    if let Some(expect_value) = db_hash {
        let salt = &expect_value[(&expect_value.chars().count() - 16)..];
        let actual_value = create_pass_hash(&hkreq.username, &hkreq.password, salt);
        if actual_value == expect_value {
            println!("Authentication succeeded for  user {}.", &hkreq.username);
            true
        } else {
            println!("Authentication failed for user {}", &hkreq.username);
            false
        }
    } else {
        println!(
            "Authentication failed for nonexistent user {}.",
            &hkreq.username
        );
        false
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
