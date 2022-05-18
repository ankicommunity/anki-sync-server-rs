use crate::{error::ApplicationError, session::Session};
use rusqlite::{params, Connection, Result};

use crate::db::fetchone;
#[allow(unused_imports)]
use anki::media::{
    files::{add_data_to_folder_uniquely, data_for_file, normalize_filename, sha1_of_data},
    sync::{hex, unicode_normalization, zip},
};
use serde_derive::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    io::Read,
    io::{self, Write},
    path::PathBuf,
};

static SYNC_MAX_BYTES: usize = (2.5 * 1024.0 * 1024.0) as usize;
static SYNC_SINGLE_FILE_MAX_BYTES: usize = 100 * 1024 * 1024;
/// open zip from vec of u8,this is a test function and will not be included into the binary.
fn _open_zip(d: Vec<u8>) -> Result<(), ApplicationError> {
    //    open zip on server

    let reader = io::Cursor::new(d);
    let mut zip = zip::ZipArchive::new(reader)?;
    let mut meta_file = zip.by_name("_meta")?;
    let mut v = vec![];
    meta_file.read_to_end(&mut v)?;
    let _map: Vec<(String, Option<String>)> = serde_json::from_slice(&v)?;
    drop(meta_file);
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name();

        if name == "_meta" {
            continue;
        }

        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)?;
        println!("{:?}", &data);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ZipRequest {
    files: Vec<String>,
}
#[derive(Debug, Serialize)]
struct ZipResponse {
    files: Vec<(String, Option<String>)>,
}
#[derive(Debug, Serialize)]
pub struct UploadChangesResult {
    pub data: Option<Vec<usize>>,
    pub err: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct MediaRecord {
    pub fname: String,
    pub usn: i32,
    pub sha1: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MediaRecordResult {
    pub data: Option<Vec<(String, i32, String)>>,
    pub err: String,
}
pub struct MediaManager {
    pub db: Connection,
    pub media_folder: PathBuf,
}

impl MediaManager {
    /// used in media sync method `uploadChanges`
    ///
    /// this will make changes to server media folder and database,
    /// e.g.delete files from media folder or add files into media folder ,it depends on client request
    ///  ,the same to media database. return count of deleted and added files and `last_usn`
    ///
    /// a part of `_meta` file of zip data from client request is as follows.It is a
    ///  vector of `filename` and `zipname` in tuple.
    ///
    /// `Some("")` or `None` means
    /// provided files to which filenames point have been deleted from client and will be deleted from
    /// server; `Some("0")` means provided files to which filenames point have been added into the client
    /// and will be added into server.
    ///
    ///  \[("paste-7cd381cbfa7a48319fae2333328863d303794b55.jpg", Some("0")),
    ///  ("paste-a4084c2983a8b7024e8f98aaa8045c41ec29e7bd.jpg", None),
    /// ("paste-f650a5de12d857ad0b51ee6afd62f697b4abf9f7.jpg", Some("2"))\]
    pub async fn adopt_media_changes_from_zip(
        &self,
        zip_data: Vec<u8>,
    ) -> Result<(usize, i32), ApplicationError> {
        let media_dir = &self.media_folder;
        let reader = io::Cursor::new(zip_data);
        let mut zip = zip::ZipArchive::new(reader)?;
        let mut meta_file = zip
            .by_name("_meta")
            .expect("Could not find '_meta' file in archive");
        let mut v = vec![];
        meta_file.read_to_end(&mut v)?;

        let d: Vec<(String, Option<String>)> = serde_json::from_slice(&v)?;

        let mut media_to_remove = vec![];
        let mut media_to_add = vec![];
        let mut fmap = HashMap::new();
        for (fname, o) in d {
            if let Some(zip_name) = o {
                //  zip_name is Some("") if
                // media file is deleted from ankidroid client
                if zip_name.is_empty() {
                    media_to_remove.push(fname);
                } else {
                    fmap.insert(zip_name, fname);
                }
            } else {
                // probably zip_name is None if  deleted from desktop client
                media_to_remove.push(fname);
            }
        }

        drop(meta_file);
        let mut usn = self.last_usn()?;
        fs::create_dir_all(&media_dir)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let name = file.name();

            if name == "_meta" {
                continue;
            }
            let real_name = match fmap.get(name) {
                None => {
                    return Err(ApplicationError::ValueNotFound(format!(
                        "Could not find name {} in fmap",
                        name
                    )))
                }
                Some(s) => s,
            };

            let mut data = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut data)?;
            //    write zip data to media folder
            usn += 1;
            let add = self.add_file(real_name, &data, usn).await?;

            media_to_add.push(add);
        }
        let processed_count = media_to_add.len() + media_to_remove.len();
        let lastusn = self.last_usn()?;
        // db ops add/delete

        if !media_to_remove.is_empty() {
            self.delete(media_to_remove.as_slice())?;
        }
        if !media_to_add.is_empty() {
            self.records_add(media_to_add)?;
        }
        Ok((processed_count, lastusn))
    }
    ///remove records from media db by filename
    fn remove_entry(&self, filename: &str) -> Result<(), ApplicationError> {
        let sql = "delete from media where fname=?";
        self.db.execute(sql, params![filename])?;
        Ok(())
    }
    /// used in media sync method `downloadFiles`
    ///
    /// read and compress local media files into a vector of bytes by a vector of file name strings (which is from client request)
    /// and return it.
    ///
    /// client requested example filenames example are as follows
    /// "{\"files\":\[\"paste-ceaa6863ee1c4ee38ed1cd3a0a2719fa934517ed.jpg\",
    /// \"sapi5js-08c91aeb-d6ae72e4-fa3faf05-eff30d1f-581b71c8.mp3\",
    /// \"sapi5js-2750d034-14d4845f-b60dc87b-afb7197f-87930ab7.mp3\"]}"
    pub fn zip_files(&self, files: ZipRequest) -> Result<Vec<u8>, ApplicationError> {
        let buf = vec![];
        let mut invalid_entries = vec![];
        let files = &files.files;
        let media_folder = &self.media_folder;
        let w = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(w);

        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut accumulated_size = 0;
        let mut map = HashMap::new();
        for (idx, file) in files.iter().enumerate() {
            if accumulated_size > SYNC_MAX_BYTES {
                break;
            }

            #[cfg(target_vendor = "apple")]
            {
                use unicode_normalization::is_nfc;
                if !is_nfc(file) {
                    // older Anki versions stored non-normalized filenames in the DB; clean them up
                    invalid_entries.push(file.as_str());
                    continue;
                }
            }

            let file_data = match data_for_file(media_folder, file) {
                Ok(data) => data,
                Err(_) => {
                    invalid_entries.push(file);
                    continue;
                }
            };

            if let Some(data) = &file_data {
                let normalized = normalize_filename(file);
                if let Cow::Owned(_) = normalized {
                    invalid_entries.push(file);
                    continue;
                }

                if data.is_empty() {
                    invalid_entries.push(file);
                    continue;
                }
                if data.len() > SYNC_SINGLE_FILE_MAX_BYTES {
                    invalid_entries.push(file);
                    continue;
                }
                accumulated_size += data.len();
                zip.start_file(format!("{}", idx), options)?;
                zip.write_all(data)?;
            }
            let in_zip_name = if file_data.is_some() {
                format!("{}", idx)
            } else {
                String::new()
            };
            map.insert(in_zip_name, file);
        }
        if !invalid_entries.is_empty() {
            // clean up invalid entries; we'll build a new zip

            for fname in invalid_entries {
                self.remove_entry(fname)?;
            }
        }

        // meta
        // {"0": "sapi5js-08c91aeb-d6ae72e4-fa3faf05-eff30d1f-581b71c8.mp3",
        //  "1": "sapi5js-2750d034-14d4845f-b60dc87b-afb7197f-87930ab7.mp3",
        // "2": "sapi5js-56393ce0-99ef886b-14d4c21f-cd7957f2-aa1cf000.mp3",}
        let meta = serde_json::to_string(&map)?;
        zip.start_file("_meta", options)?;
        zip.write_all(meta.as_bytes())?;

        let w = zip.finish()?;

        Ok(w.into_inner())
    }
    /// open media db and return  an instance of `MediaManager`
    pub fn new(session: &Session) -> Result<Self> {
        let (media_db, media_folder) = session.media_dir_db();
        let db = Connection::open(media_db)?;
        db.execute_batch(include_str!("schema.sql"))?;
        Ok(MediaManager { db, media_folder })
    }
    /// used in media sync method `mediaChanges`
    ///
    ///  return records whose usns start from client `last_usn` to server `last_usn`,and there is
    /// a prerequisite that client `last_usn` < server `last_usn`
    ///
    /// ## example
    /// assme client lastusn is 0,server lastusn 135
    /// then the server will return 135 records
    ///
    /// |filename|usn|checksum|
    /// |---|---|---|
    /// |rec1634015317.mp3| 135 |None|
    /// |sapi5js-42ecd8a6-427ac916-0ba420b0-b1c11b85-f20d5990.mp3| 134| None|
    /// |paste-c9bde250ab49048b2cfc90232a3ae5402aba19c3.jpg| 133| c9bde250ab49048b2cfc90232a3ae5402aba19c3|
    /// |paste-d8d989d662ae46a420ec5d440516912c5fbf2111.jpg| 132| d8d989d662ae46a420ec5d440516912c5fbf2111|
    /// ...
    pub fn changes(&self, last_usn: i32) -> Result<Vec<(String, i32, String)>, ApplicationError> {
        let sql = "select fname,usn,csum from media order by usn desc limit ?";
        let diff_usn = self.last_usn()? - last_usn;
        let mut stmt = self.db.prepare(sql)?;
        let mut rs = stmt.query(params![diff_usn])?;
        let mut v: Vec<(String, i32, String)> = vec![];
        while let Some(r) = rs.next()? {
            v.push((r.get(0)?, r.get(1)?, r.get(2).map_or(String::new(), |e| e)));
        }
        Ok(v)
    }
    /// count all media items(records,not include already deleted records whose csums are NULL) from media db
    pub fn count(&self) -> Result<u32, ApplicationError> {
        let sql = "SELECT count() FROM media WHERE csum IS NOT NULL";
        fetchone(&self.db, sql, None)?
            .ok_or_else(|| ApplicationError::ValueNotFound("count not found in media".to_string()))
    }
    /// `usn` is something like `index`,ehich starts from 1, to a database. Every time a record is added into the db,
    /// `usn` will be incremented.`last_usn` is the usn of the nrely inserted record
    /// since last client media change request.In the following example,`last_usn=4`
    ///
    /// ## example media database records
    /// |filename|usn|checksum|
    /// |---|---|---|
    /// |2022-03-30_114732.jpg |1|772d832009fea3ffeb63306f1016243b6cc170c3|
    /// |_A4beak.jpg|2|6ec66d655b308cedd207f049f570f25b1e5d0007|
    /// |_A4bend.jpg|3|cd8dc6efce26b4aa580ba292d21cc6caad308542|
    /// |_A4chain.jpg|4|03b2df725edfa17e7a022bdadf7c9476ebe14e70|
    pub fn last_usn(&self) -> Result<i32, ApplicationError> {
        let sql = "SELECT usn FROM media ORDER BY usn DESC";
        let ls: Option<i32> = fetchone(&self.db, sql, None)?;
        if let Some(usn) = ls {
            Ok(usn)
        } else {
            Ok(0)
        }
    }
    /// insert records to media db while client add media files and sync to server
    pub fn records_add(&self, adds: Vec<MediaRecord>) -> Result<(), ApplicationError> {
        let sql = "INSERT OR REPLACE INTO media VALUES (?,?,?)";
        for i in adds {
            self.db.execute(sql, params![i.fname, i.usn, i.sha1])?;
        }
        Ok(())
    }
    /// delete files from media  folder by filenames and update db by setting `csum=null` and incrementing `usn`
    pub fn delete(&self, rms: &[String]) -> Result<(), ApplicationError> {
        for i in rms {
            let fpath = self.media_folder.join(i);

            if fpath.exists() {
                fs::remove_file(fpath)?;
            }
            let sql = "UPDATE media SET csum = NULL, usn = ? WHERE fname = ?";
            let usn = self.last_usn()? + 1;
            self.db.execute(sql, params![usn, i])?;
        }
        Ok(())
    }
    /// write uncompressed zip data to local media files and return `MediaRecord`
    pub async fn add_file(
        &self,
        fname: &str,
        data: &[u8],
        usn: i32,
    ) -> Result<MediaRecord, ApplicationError> {
        let media_folder = &self.media_folder;
        let sha1 = sha1_of_data(data);
        let normalized = normalize_filename(fname);

        // if the filename is already valid, we can write the file directly
        let (_renamed_from, _path) = if let Cow::Borrowed(_) = normalized {
            let path = media_folder.join(normalized.as_ref());

            async_std::fs::write(&path, data).await?;
            (None, path)
        } else {
            // ankiweb sent us a non-normalized filename, so we'll rename it
            let new_name = add_data_to_folder_uniquely(media_folder, fname, data, sha1)?;

            (
                Some(fname.to_string()),
                media_folder.join(new_name.as_ref()),
            )
        };

        Ok(MediaRecord {
            fname: normalized.to_string(),
            usn,
            sha1: hex::encode(sha1),
        })
    }
}
