use simple_server::{Method, Server, StatusCode, Request, ResponseBuilder, ResponseResult};
use libcommonplace::{Note, open_db, get_tag_tree, rename_note};
use rust_embed::RustEmbed;
use uuid::Uuid;
use rusqlite::params;
use std::str::FromStr;

#[derive(RustEmbed)]
#[folder = "../commonplace_gui_client/static/"]
struct StaticFiles;

fn make_404(response: &mut ResponseBuilder) -> ResponseResult {
    response.status(StatusCode::NOT_FOUND);
    Ok(response.body("<h1>404</h1><p>Not found!<p>".as_bytes().to_vec())?)
}

fn handle_static(request: &Request<Vec<u8>>, response: &mut ResponseBuilder) -> ResponseResult {
    let mimetype = match request.uri().path().split(".").last() {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        _ => "text/plain",
    };

    match StaticFiles::get(&request.uri().path()[1..]) {
        Some(x) => Ok(response.header("Content-Type", mimetype).body(x[..].to_vec())?),
        None => make_404(response),
    }
}

fn handle_show_tree(response: &mut ResponseBuilder) -> ResponseResult {
    let db = open_db().unwrap();
    let tree = get_tag_tree(&db).unwrap();
    Ok(response.header("Content-Type", "application/json").body(serde_json::to_vec(&tree).unwrap())?)
}

fn handle_get_blob(response: &mut ResponseBuilder, hash: &str) -> ResponseResult {
    if !hash.chars().all(|c| (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))  {
        return make_404(response);
    }
    if let Ok(contents) = std::fs::read(hash) {
        Ok(response.header("Content-Type", "application/octet-stream").body(contents)?)
    } else {
        make_404(response)
    }
}

fn handle_get_note(response: &mut ResponseBuilder, uuid: &str) -> ResponseResult {
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
        Ok(response.header("Content-Type", "application/json").body(serde_json::to_vec(&note).unwrap())?)
    } else {
        make_404(response)
    }
}

fn handle_rename_note(response: &mut ResponseBuilder, request: &Request<Vec<u8>>, uuid: &str) -> ResponseResult {
    if let Ok(uuid) = Uuid::from_str(uuid) {
        let db = open_db().unwrap();
        rename_note(&db, uuid, String::from_utf8(request.body().to_vec()).unwrap());
        Ok(response.body(vec![]).unwrap())
    } else {
        make_404(response)
    }
}

fn main() {
    let port = 38841;
    let bind_addr = "127.0.0.1";

    //std::thread::spawn(move || {
        let server = Server::new(|request, mut response| {
            let path: Vec<&str> = request.uri().path().split("/").filter(|x| *x != "").collect();
            match (request.method(), &path[..]) {
                (&Method::GET, &["api", "showtree"]) => handle_show_tree(&mut response),
                (&Method::GET, &["api", "blob", hash]) => handle_get_blob(&mut response, hash),
                (&Method::GET, &["api", "note", uuid]) => handle_get_note(&mut response, uuid),
                (&Method::GET, _) => handle_static(&request, &mut response),
                (&Method::POST, &["api", "note", uuid, "rename"]) => handle_rename_note(&mut response, &request, uuid),
                (_, _) => make_404(&mut response),
            }
        });

        server.listen(bind_addr, &format!("{}", port));
    //});

    /*
    web_view::builder()
        .title("Commonplace")
        .content(web_view::Content::Url(format!("http://{}:{}/index.html", bind_addr, port)))
        .user_data(0)
        .invoke_handler(|_webview, _arg| Ok(()))
        .run()
        .unwrap();
    */
}
