#![allow(clippy::wildcard_imports)]

use seed::{prelude::*, *};

use uuid::Uuid;
use enclose::enc;

use std::collections::HashMap;

use libcommonplace_types::{Note, TagTree};

fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.after_next_render(|_| { start_slate(); });
    orders.send_msg(Msg::RequestUpdateTagTree);
    Model {
        tag_tree: None,
        tag_tree_folds: HashMap::new(),
        current_note: None,
    }
}

struct Model {
    tag_tree: Option<Vec<TagTree>>,
    tag_tree_folds: HashMap<Uuid, bool>,
    current_note: Option<Note>,
}

enum Msg {
    RequestUpdateTagTree,
    UpdateTagTree(Vec<TagTree>),
    ToggleTag(Uuid),
    OpenNote(Note),
    NoteBlobLoaded((Note, String)),
    RenameNote((Option<Uuid>, String)),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::RequestUpdateTagTree => {
            orders.perform_cmd(async {
                get_tag_tree().await.map(|t| Msg::UpdateTagTree(t)).ok()
            });
        },
        Msg::UpdateTagTree(tag_tree) => {
            model.tag_tree = Some(tag_tree);
        },
        Msg::ToggleTag(uuid) => {
            *model.tag_tree_folds.entry(uuid).or_insert(true) ^= true;
        },
        Msg::OpenNote(note) => {
            orders.perform_cmd(enc!((note) async {
                get_blob(&hex::encode(&note.hash)).await.map(|b| Msg::NoteBlobLoaded((note, b))).ok()
            }));
        },
        Msg::NoteBlobLoaded((note, blob)) => {
            update_slate(&blob);
            model.current_note = Some(note);
        },
        Msg::RenameNote((uuid, name)) => {
            if let Some(uuid) = uuid.or(model.current_note.as_ref().map(|x| x.id).or(None)) {
                orders.perform_cmd(async move {
                    rename_note(uuid, name).await;
                    Msg::RequestUpdateTagTree
                });
            }
        },
    }
}

async fn get_tag_tree() -> Result<Vec<TagTree>, ()> {
    let bytes = Request::new("/api/showtree")
        .method(Method::Get)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?
        .bytes().map_err(|e| { log!(e) }).await?;
    serde_json::from_slice(&bytes[..]).map_err(|e| { log!(e) })
}

async fn get_blob(hash: &str) -> Result<String, ()> {
    let bytes = Request::new(format!("/api/blob/{}", hash))
        .method(Method::Get)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?
        .bytes().map_err(|e| { log!(e) }).await?;
    String::from_utf8(bytes).map_err(|e| { log!(e) })
}

async fn rename_note(uuid: Uuid, name: String) -> Result<(), ()> {
    Request::new(format!("/api/note/{}/rename", uuid))
        .method(Method::Post)
        .body(name.into())
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?;
    Ok(())
}

fn view(model: &Model) -> Node<Msg> {
    div![
        div![
            id!["sidebar"],
            IF![
                model.tag_tree.is_some() =>
                tree_view(model.tag_tree.as_ref().unwrap(), &model.tag_tree_folds)
            ],
        ],
        div![
            id!["noteview"],
            input![
                attrs! {
                    At::Value => model.current_note.as_ref().map(|note| note.name.as_str()).unwrap_or("");
                },
                ev(Ev::Blur, |event| {
                    let name = event.target().unwrap()
                        .unchecked_into::<web_sys::HtmlInputElement>()
                        .value();
                    Msg::RenameNote((None, name))
                })
            ],
            div![id!["editor"]],
        ],
    ]
}

fn tree_view(tag_tree: &Vec<TagTree>, tag_tree_folds: &HashMap<Uuid, bool>) -> Node<Msg> {
    ul![
        tag_tree.iter().map(|tag| {
            li![
                IF![!tag_tree_folds.get(&tag.id).unwrap_or(&true) => C!["tree-closed"]],
                button![
                    C!["tag"],
                    &tag.name,
                    ev(Ev::Click, enc!((&tag.id => id) move |_| Msg::ToggleTag(id))),
                ],
                tree_view(&tag.children, tag_tree_folds),
                ul![
                    tag.notes.iter().map(|note| {
                        li![
                            button![
                                C!["note"],
                                &note.name,
                                ev(Ev::Click, enc!((note) move |_| Msg::OpenNote(note))),
                            ],
                        ]
                    }).collect::<Vec<Node<Msg>>>(),
                ],
            ]
        }).collect::<Vec<Node<Msg>>>()
    ]
}

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}

#[wasm_bindgen]
extern "C" {
    fn start_slate();
    #[wasm_bindgen(js_namespace = window)]
    fn update_slate(_: &str);
}
