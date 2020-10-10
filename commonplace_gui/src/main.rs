use simple_server::{Method, Server, StatusCode, Request, ResponseBuilder, ResponseResult};
use libcommonplace::{Connection, open_db, get_tag_tree};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/static"]
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

fn main() {
    let port = 38841;
    let bind_addr = "127.0.0.1";

    std::thread::spawn(move || {
        let server = Server::new(|request, mut response| {
            let path: Vec<&str> = request.uri().path().split("/").filter(|x| *x != "").collect();
            match (request.method(), &path[..]) {
                (&Method::GET, &["api", "showtree"]) => handle_show_tree(&mut response),
                (&Method::GET, _) => handle_static(&request, &mut response),
                (_, _) => make_404(&mut response),
            }
        });

        server.listen(bind_addr, &format!("{}", port));
    });

    web_view::builder()
        .title("Commonplace")
        .content(web_view::Content::Url(format!("http://{}:{}/index.html", bind_addr, port)))
        .user_data(0)
        .invoke_handler(|_webview, _arg| Ok(()))
        .run()
        .unwrap();
}
