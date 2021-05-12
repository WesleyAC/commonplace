use rouille::{Request, Response};
use libcommonplace::{NoteId, TagId, Note, add_note, open_db, get_all_notes, get_untagged_notes, get_tag_tree, rename_note, update_note_bytes, tag_note_by_uuid};
use rust_embed::RustEmbed;
use uuid::Uuid;
use rusqlite::params;
use std::str::FromStr;
use std::io::Read;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "../commonplace_gui_client/static/"]
struct StaticFiles;

fn handle_static(path: String) -> Response {
    let mimetype = match path.split(".").last() {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        _ => "text/plain",
    };

    match StaticFiles::get(&path) {
        Some(x) => Response::from_data(mimetype, x[..].to_vec()),
        None => Response::empty_404(),
    }
}

fn handle_show_tree() -> Response {
    let db = open_db().unwrap();
    let tree = get_tag_tree(&db).unwrap();
    Response::from_data("application/json", serde_json::to_vec(&tree).unwrap())
}

fn handle_get_blob(hash: &str) -> Response {
    if !hash.chars().all(|c| (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))  {
        return Response::empty_404();
    }
    if let Ok(contents) = std::fs::read(hash) {
        Response::from_data("application/octet-stream", contents)
    } else {
        Response::empty_404()
    }
}

fn handle_get_note(uuid: &str) -> Response {
    let db = open_db().unwrap();
    if let Ok(uuid) = Uuid::from_str(uuid) {
        let mut note_query = db.prepare("SELECT * FROM Notes WHERE id = ?1").unwrap();
        let note = note_query.query_row(params![uuid], |row| {
            let mut hash: [u8; 32] = [0; 32];
            hash.copy_from_slice(&row.get::<&str, Vec<u8>>("hash")?[..]);
            Ok(Note {
                id: uuid,
                hash,
                name: row.get("name")?,
                mimetype: row.get("mimetype")?,
            })
        }).unwrap();
        Response::from_data("application/json", serde_json::to_vec(&note).unwrap())
    } else {
        Response::empty_404()
    }
}

fn handle_rename_note(name: Vec<u8>, uuid: &str) -> Response {
    if let Ok(uuid) = Uuid::from_str(uuid) {
        let db = open_db().unwrap();
        rename_note(&db, uuid, String::from_utf8(name).unwrap());
        Response::empty_204()
    } else {
        Response::empty_404()
    }
}

fn handle_update_note(contents: Vec<u8>, uuid: &str) -> Response {
    if let Ok(uuid) = Uuid::from_str(uuid) {
        let db = open_db().unwrap();
        update_note_bytes(&db, uuid, contents);
        Response::empty_204()
    } else {
        Response::empty_404()
    }
}

fn handle_get_notes() -> Response {
    let db = open_db().unwrap();
    if let Ok(notes) = get_all_notes(&db) {
        Response::from_data("application/json", serde_json::to_vec(&notes).unwrap())
    } else {
        Response::empty_404()
    }
}

fn handle_get_untagged_notes() -> Response {
    let db = open_db().unwrap();
    if let Ok(notes) = get_untagged_notes(&db) {
        Response::from_data("application/json", serde_json::to_vec(&notes).unwrap())
    } else {
        Response::empty_404()
    }
}

fn handle_new_note() -> Response {
    let db = open_db().unwrap();
    if let Ok(uuid) = add_note(&db, "new_note".to_string(), PathBuf::from(r"/dev/null")) {
        Response::from_data("application/json", serde_json::to_vec(&uuid).unwrap())
    } else {
        Response::empty_404()
    }
}

fn handle_note_add_tag(note_id: &str, tag_id: &str) -> Response {
    let db = open_db().unwrap();
    let note_id = Uuid::from_str(note_id);
    let tag_id = Uuid::from_str(tag_id);
    if let (Ok(note_id), Ok(tag_id)) = (note_id, tag_id) {
        tag_note_by_uuid(&db, note_id, tag_id);
        Response::empty_204()
    } else {
        Response::empty_404()
    }
}

#[macro_use]
extern crate rouille;

fn main() {
    rouille::start_server("localhost:38841", move |request| {
        let url = request.url();
        let path: Vec<&str> = url.split("/").filter(|x| *x != "").collect();
        match (request.method(), &path[..]) {
            ("GET", &["api", "showtree"]) => handle_show_tree(),
            ("GET", &["api", "notes"]) => handle_get_notes(),
            ("GET", &["api", "notes", "untagged"]) => handle_get_untagged_notes(),
            ("GET", &["api", "blob", hash]) => handle_get_blob(hash),
            ("GET", &["api", "note", uuid]) => handle_get_note(uuid),
            ("GET", path) => handle_static(path.join("/")),
            ("POST", &["api", "note", "new"]) => handle_new_note(),
            ("POST", &["api", "note", note_id, "tag", tag_id]) => handle_note_add_tag(note_id, tag_id),
            ("POST", &["api", "note", uuid, "rename"]) => {
                let mut body = vec![];
                request.data().unwrap().read_to_end(&mut body);
                handle_rename_note(body, uuid)
            },
            ("POST", &["api", "note", uuid]) => {
                let mut body = vec![];
                request.data().unwrap().read_to_end(&mut body);
                handle_update_note(body, uuid)
            },

            _ => rouille::Response::empty_404()
        }
    });
}
