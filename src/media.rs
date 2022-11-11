use crate::db::fetchone;
use crate::{error::ApplicationError, session::Session};
use actix_web::HttpResponse;
#[allow(unused_imports)]
use anki::media::files::{
    add_data_to_folder_uniquely, data_for_file, normalize_filename, sha1_of_data,
};
use async_trait::async_trait;
use media_structs::*;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    io::Read,
    io::{self, Write},
    path::PathBuf,
};
#[cfg(target_vendor = "apple")]
use unicode_normalization;
/// these structs are copied from `anki/rslib/media/sync.rs`
pub mod media_structs {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    pub(crate) struct FinalizeRequest {
        pub(crate) local: u32,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct FinalizeResponse {
        pub(crate) data: Option<String>,
        pub(crate) err: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct RecordBatchRequest {
        pub(crate) last_usn: i32,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct SyncBeginResponse {
        #[serde(rename = "sk")]
        pub(crate) sync_key: String,
        pub(crate) usn: i32,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) struct SyncBeginResult {
        pub(crate) data: Option<SyncBeginResponse>,
        pub(crate) err: String,
    }
}
pub static MOPERATIONS: [&str; 5] = [
    "begin",
    "mediaChanges",
    "mediaSanity",
    "uploadChanges",
    "downloadFiles",
];
static SYNC_MAX_BYTES: usize = (2.5 * 1024.0 * 1024.0) as usize;
static SYNC_SINGLE_FILE_MAX_BYTES: usize = 100 * 1024 * 1024;

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
#[async_trait(?Send)]
trait MediaSyncMethod {
    async fn begin(&self, session: Session) -> Result<HttpResponse, ApplicationError>;
    async fn upload_changes(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError>;
    async fn download_files(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError>;
    async fn media_changes(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError>;
    async fn media_sanity(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError>;
}
#[async_trait(?Send)]
impl MediaSyncMethod for MediaManager {
    async fn begin(&self, session: Session) -> Result<HttpResponse, ApplicationError> {
        let lastusn = self.last_usn()?;
        let sbr = SyncBeginResult {
            data: Some(SyncBeginResponse {
                sync_key: session.skey(),
                usn: lastusn,
            }),
            err: String::new(),
        };
        Ok(HttpResponse::Ok().json(sbr))
    }
    /// override or update media state of server by bringing in media state of client .
    ///
    /// for examle,the server will follow the operation if the client(App) perform an action
    /// of deleting cards (which contain media files that have to be deleted)
    async fn upload_changes(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError> {
        let (procs_cnt, lastusn) = match self.adopt_media_changes_from_zip(data).await {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let upres = UploadChangesResult {
            data: Some(vec![procs_cnt, lastusn as usize]),
            err: String::new(),
        };
        Ok(HttpResponse::Ok().json(upres))
    }
    async fn download_files(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError> {
        let filename_entries: ZipRequest = serde_json::from_slice(&data)?;
        let d = self.zip_files(filename_entries)?;

        Ok(HttpResponse::Ok().body(d))
    }
    /// return newer changes of the server compared to the client.
    ///
    /// Here we use last_usn to compare.If last_usn in server is greater than that in client,
    /// the range of records from db from last_usn in client to that in server.
    ///
    /// # example
    /// Assume we add two cards(one media file per card) in thr client and casually delete
    /// one of them,then sync to server.And the return value is as follows.
    ///
    /// `[("2.png", 2, "6dd5c9226ca51d53da1ec53edbe3ca030b47b579"), ("1.png", 1, "fa78ed3708d36e682db7b0965f2c603
    /// 91e227256")]`
    ///
    /// the `last_usn` s are 0(client),2(server).
    ///
    /// Then we check media and delete unused files in the client,sync to server.As the values(last_usns of both server and client) are equal,
    /// so return empty value.
    ///
    /// This method is often called after `upload_changes`.
    async fn media_changes(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError> {
        let rbr: RecordBatchRequest = serde_json::from_slice(&data)?;
        let client_lastusn = rbr.last_usn;
        let server_lastusn = self.last_usn()?;
        // debug use to print usns
        // println!(
        //     "client_last_usn {},server_lastusn {}",
        //     &client_lastusn, &server_lastusn
        // );
        let d = if client_lastusn < server_lastusn || client_lastusn == 0 {
            let mut chges = self.changes(client_lastusn, server_lastusn)?;
            chges.reverse();
            MediaRecordResult {
                data: Some(chges),
                err: String::new(),
            }
        } else {
            MediaRecordResult {
                data: Some(Vec::new()),
                err: String::new(),
            }
        };

        Ok(HttpResponse::Ok().json(d))
    }
    async fn media_sanity(&self, data: Vec<u8>) -> Result<HttpResponse, ApplicationError> {
        let locol: FinalizeRequest = serde_json::from_slice(&data)?;
        let res = if self.media_count()? == locol.local {
            "OK"
        } else {
            "FAILED"
        };
        let result = FinalizeResponse {
            data: Some(res.to_owned()),
            err: String::new(),
        };
        Ok(HttpResponse::Ok().json(result))
    }
}
impl MediaManager {
    /// media sync methods handler ,e.g. `begin`,`uploadChanges` ...
    pub async fn media_sync(
        &self,
        method: &str,
        session: Session,
        data: Vec<u8>,
    ) -> Result<HttpResponse, ApplicationError> {
        match method {
            "begin" => self.begin(session).await,
            "uploadChanges" => self.upload_changes(data).await,
            "mediaChanges" => self.media_changes(data).await,
            "downloadFiles" => self.download_files(data).await,
            "mediaSanity" => self.media_sanity(data).await,
            _ => Ok(HttpResponse::Ok().finish()),
        }
    }
    /// used in media sync method `uploadChanges`
    ///
    /// this will make changes to server media folder and database according to meta data of zip from client,
    /// e.g.delete files from media folder or add files into media folder ,it depends on client request
    ///  ,the same to media database. return count of deleted and added files and `last_usn`
    ///
    /// # example (sample meta contents of zip from clients.)
    ///  Assume we add two cards(one media file per card) in thr client and casually delete
    /// one of them,then sync to server.And the meta data is as follows.
    ///
    ///  `[("1.png", Some("0")), ("2.png", Some("1"))]`.the tupple pair of the vector represent
    /// real media filename and zip name(which is in numerical order) This means these two media files will be uploaded
    /// to the server even though one card has been deleted.
    ///
    /// After checking and delete unused files in client and then sync,we find that the meta data is as follows.
    ///
    /// `[("2.png", None)]`（Anki desktop） or `[("2.png", "")]` (Ankidroid).This means one media file named `2.png` will be deleted from the server.
    ///
    /// And the output of media db after finishing sync is:
    ///
    /// |filename|usn|csum(check sum)|
    /// |---|---|---|
    /// |1.png|1|fa78ed3708d36e682db7b0965f2c60391e227256|
    /// |2.png|3|null|
    ///
    /// `Some("")` or `None` means
    /// provided files to which filenames point have been deleted from client and will be deleted from
    /// server; `Some("0")` means provided files to which filenames point have been added into the client
    /// and will be added into server.
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
        // debug use to display meta contents of zip.
        // println!("{:?}", &d);
        let mut media_to_remove = vec![];
        let mut media_to_add = vec![];
        let mut fmap = HashMap::new();
        for (fname, o) in d {
            if let Some(zip_name) = o {
                //  zip_name is Some("") if
                // media file is deleted from ankidroid client and checking media and
                // deleting unused files are used.
                if zip_name.is_empty() {
                    media_to_remove.push(fname);
                } else {
                    fmap.insert(zip_name, fname);
                }
            } else {
                // zip_name is None if  client check media and delete unused files and sync to server.
                media_to_remove.push(fname);
            }
        }

        drop(meta_file);
        let mut usn = self.last_usn()?;
        fs::create_dir_all(media_dir)?;
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
    /// read and compress local media files into zip bytes by a vector of file name strings (which is from client request)
    /// and return it.
    ///
    /// client requested example filenames example are as follows:
    ///
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

        // write follwoing meta data to zip
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
        db.execute_batch(include_str!("create_media.sql"))?;
        Ok(MediaManager { db, media_folder })
    }
    /// used in media sync method `mediaChanges`
    ///
    ///  return records whose usns start from client `last_usn` to server `last_usn`,and there is
    /// a prerequisite that client `last_usn` < server `last_usn`
    ///
    /// ## example
    /// assume client lastusn is 0,server lastusn 135
    /// then the server will return 135 records
    ///
    /// |filename|usn|checksum|
    /// |---|---|---|
    /// |rec1634015317.mp3| 135 |None|
    /// |sapi5js-42ecd8a6-427ac916-0ba420b0-b1c11b85-f20d5990.mp3| 134| None|
    /// |paste-c9bde250ab49048b2cfc90232a3ae5402aba19c3.jpg| 133| c9bde250ab49048b2cfc90232a3ae5402aba19c3|
    /// |paste-d8d989d662ae46a420ec5d440516912c5fbf2111.jpg| 132| d8d989d662ae46a420ec5d440516912c5fbf2111|
    /// ...
    pub fn changes(
        &self,
        client_last_usn: i32,
        server_last_usn: i32,
    ) -> Result<Vec<(String, i32, String)>, ApplicationError> {
        let sql = "select fname,usn,csum from media order by usn desc limit ?";
        let diff_usn = server_last_usn - client_last_usn;
        let mut stmt = self.db.prepare(sql)?;
        let mut rs = stmt.query(params![diff_usn])?;
        let mut v: Vec<(String, i32, String)> = vec![];
        while let Some(r) = rs.next()? {
            v.push((r.get(0)?, r.get(1)?, r.get(2).map_or(String::new(), |e| e)));
        }
        // debug use to display result
        // println!("changes {:?}", &v);
        Ok(v)
    }
    /// count all media items/records(not include already deleted records whose csums are NULL)
    ///  from media db at server side
    pub fn media_count(&self) -> Result<u32, ApplicationError> {
        let sql = "SELECT count() FROM media WHERE csum IS NOT NULL";
        fetchone(&self.db, sql, None)?
            .ok_or_else(|| ApplicationError::ValueNotFound("count not found in media".to_string()))
    }
    /// `usn` is something like `index`,ehich starts from 1, to a database. Every time a record is added into the db,
    /// `usn` will be incremented.It can be intepreted as `universal serial number`.
    /// `last_usn` is the usn of the newest inserted record
    /// since last client media change request.
    ///
    /// ## example
    /// assume there are 4 records in media database
    /// In the following example,`last_usn=4`
    ///
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
    /// loop operation to delete files one by one from media folder by a vec of filenames and
    ///
    /// update db by setting `csum=null` and `usn` plus 1 after corresponding file has been deleted
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
    /// write io buffer, uncompressed from zip, to one media file on server side and return `MediaRecord`
    ///
    /// MediaRecord is as follows
    /// ```
    /// pub struct MediaRecord {
    /// pub fname: String,
    /// pub usn: i32,
    /// pub sha1: String,
    /// }
    /// ```
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
