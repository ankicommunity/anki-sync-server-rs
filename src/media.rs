use rusqlite::{params, Connection, OptionalExtension, Result};

use crate::error::ApplicationError;
#[allow(unused_imports)]
use anki::media::{
    files::{add_data_to_folder_uniquely, data_for_file, hex, normalize_filename, sha1_of_data},
    sync::{unicode_normalization, zip},
};
use serde_derive::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

static SYNC_MAX_BYTES: usize = (2.5 * 1024.0 * 1024.0) as usize;
static SYNC_SINGLE_FILE_MAX_BYTES: usize = 100 * 1024 * 1024;
/// open media db
pub fn open_media_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(include_str!("schema.sql"))?;
    Ok(db)
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
    fn remove_entry(&self, filename: &str) -> Result<(), ApplicationError> {
        let sql = "delete from media where fname=?";
        self.db.execute(sql, params![filename])?;
        Ok(())
    }
    pub fn zip_files(&self, files: ZipRequest) -> Result<Option<Vec<u8>>, ApplicationError> {
        let buf = vec![];
        let mut invalid_entries = vec![];
        let files = &files.files;
        let media_folder = &self.media_folder;
        let w = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(w);

        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut accumulated_size = 0;
        // let mut entries = vec![];
        let mut map = HashMap::new();
        for (idx, file) in files.iter().enumerate() {
            if accumulated_size > SYNC_MAX_BYTES {
                break;
            }

            #[cfg(target_vendor = "apple")]
            {
                use unicode_normalization::is_nfc;
                if !is_nfc(&file) {
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

        Ok(Some(w.into_inner()))
    }

    pub fn new<P, P2>(media_folder: P, media_db: P2) -> Result<Self>
    where
        P: Into<PathBuf>,
        P2: AsRef<Path>,
    {
        let db = open_media_db(media_db.as_ref())?;
        Ok(MediaManager {
            db,
            media_folder: media_folder.into(),
        })
    }
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
    /// count all records on media db
    pub fn count(&self) -> Result<u32, ApplicationError> {
        let sql = "SELECT count() FROM media WHERE csum IS NOT NULL";
        let r = self
            .db
            .query_row(sql, params![], |row| row.get(0))
            .optional();
        match r {
            Ok(ret) => Ok(ret.unwrap()),
            Err(e) => Err(ApplicationError::Sqlite(e)),
        }
    }
    pub fn last_usn(&self) -> Result<i32, ApplicationError> {
        let sql = "SELECT usn FROM media ORDER BY usn DESC";
        let r = self
            .db
            .query_row(sql, params![], |row| row.get(0))
            .optional();
        match r {
            Ok(ret) => {
                if let Some(usn) = ret {
                    Ok(usn)
                } else {
                    Ok(0)
                }
            }
            Err(e) => Err(ApplicationError::Sqlite(e)),
        }
    }
    /// insert records to media db
    pub fn records_add(&self, adds: Vec<MediaRecord>) -> Result<(), ApplicationError> {
        let sql = "INSERT OR REPLACE INTO media VALUES (?,?,?)";
        for i in adds {
            self.db.execute(sql, params![i.fname, i.usn, i.sha1])?;
        }
        Ok(())
    }
    /// delete folder file and update db by set  csum=null
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
    /// write zip data to media folder
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
            sha1: Some(sha1).map(hex::encode).unwrap(),
        })
    }
}
