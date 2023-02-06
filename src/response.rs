use actix_web::HttpResponse;
// reference: https://github.com/ankicommunity/anki-core/blob/ae8f44d4b30f6e9f9c9aa8f0a7694d8cca583316/rslib/src/sync/response.rs
use anki::sync::request::header_and_stream::encode_zstd_body;
use anki::sync::response::ORIGINAL_SIZE;
use anki::sync::version::SyncVersion;
pub fn make_response(data: Vec<u8>, sync_version: SyncVersion) -> actix_web::HttpResponse {
    if sync_version.is_zstd() {
        // construct response from header and body
        let header = (&ORIGINAL_SIZE, data.len().to_string());
        let body = encode_zstd_body(data);
        HttpResponse::Ok().append_header(header).streaming(body)
    } else {
        HttpResponse::Ok().body(data)
    }
}
