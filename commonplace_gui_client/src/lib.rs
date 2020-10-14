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
        notes: HashMap::new(),
        tag_tree_folds: HashMap::new(),
        current_note: None,
    }
}

struct Model {
    tag_tree: Option<Vec<TagTree>>,
    notes: HashMap<Uuid, Note>,
    tag_tree_folds: HashMap<Uuid, bool>,
    current_note: Option<Uuid>,
}

enum Msg {
    RequestUpdateTagTree,
    UpdateTagTree((Vec<TagTree>, HashMap<Uuid, Note>)),
    ToggleTag(Uuid),
    OpenNote(Uuid),
    NoteBlobLoaded(String),
    RenameNote((Option<Uuid>, String)),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::RequestUpdateTagTree => {
            orders.perform_cmd(async {
                get_tag_tree().await.map(|t| Msg::UpdateTagTree(t)).ok()
            });
        },
        Msg::UpdateTagTree((tag_tree, notes)) => {
            model.tag_tree = Some(tag_tree);
            model.notes = notes;
        },
        Msg::ToggleTag(uuid) => {
            *model.tag_tree_folds.entry(uuid).or_insert(true) ^= true;
        },
        Msg::OpenNote(note) => {
            model.current_note = Some(note);
            if let Some(hash) = model.notes.get(&note).map(|x| x.hash) {
                orders.perform_cmd(enc!((hash) async move {
                    get_blob(&hex::encode(&hash)).await.map(|b| Msg::NoteBlobLoaded(b)).ok()
                }));
            }
        },
        Msg::NoteBlobLoaded(blob) => {
            update_slate(&blob);
        },
        Msg::RenameNote((uuid, name)) => {
            if let Some(uuid) = uuid.or(model.current_note.or(None)) {
                orders.perform_cmd(async move {
                    rename_note(uuid, name).await;
                    Msg::RequestUpdateTagTree
                });
            }
        },
    }
}

async fn get_tag_tree() -> Result<(Vec<TagTree>, HashMap<Uuid, Note>), ()> {
    let bytes = Request::new("/api/showtree")
        .method(Method::Get)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?
        .bytes().map_err(|e| { log!(e) }).await?;
    let tree = serde_json::from_slice(&bytes[..]).map_err(|e| { log!(e) })?;

    let bytes = Request::new("/api/notes")
        .method(Method::Get)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?
        .bytes().map_err(|e| { log!(e) }).await?;
    let notes = serde_json::from_slice(&bytes[..]).map_err(|e| { log!(e) })?;

    Ok((tree, notes))
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
                tree_view(model.tag_tree.as_ref().unwrap(), &model.tag_tree_folds, &model.notes)
            ],
        ],
        div![
            id!["noteview"],
            input![
                attrs! {
                    At::Value => {
                        if let Some(uuid) = model.current_note {
                            model.notes.get(&uuid).map(|x| x.name.as_str()).unwrap_or("")
                        } else {
                            ""
                        }
                    };
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

fn tree_view(tag_tree: &Vec<TagTree>, tag_tree_folds: &HashMap<Uuid, bool>, notes: &HashMap<Uuid, Note>) -> Node<Msg> {
    ul![
        tag_tree.iter().map(|tag| {
            li![
                IF![!tag_tree_folds.get(&tag.id).unwrap_or(&true) => C!["tree-closed"]],
                button![
                    C!["tag"],
                    &tag.name,
                    ev(Ev::Click, enc!((&tag.id => id) move |_| Msg::ToggleTag(id))),
                ],
                tree_view(&tag.children, tag_tree_folds, notes),
                ul![
                    tag.notes.iter().map(|note| {
                        li![
                            button![
                                C!["note"],
                                notes.get(&note).map(|x| x.name.as_str()),
                                //&note,
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
