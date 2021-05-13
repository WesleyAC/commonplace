#![allow(clippy::wildcard_imports)]

use seed::{prelude::*, *};

use enclose::enc;

use std::collections::HashMap;

use libcommonplace_types::{NoteId, TagId, Note, TagTree, get_tags_for_note, get_tag_name, get_tag_by_full_name};

fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.stream(streams::window_event(Ev::KeyDown, |event| {
        Msg::KeyPressed(event.unchecked_into())
    }));
    orders.send_msg(Msg::RequestUpdateTagTree);
    orders.after_next_render(|_| { start_slate(); });
    Model {
        tag_tree: None,
        tag_tree_folds: HashMap::new(),
        sidebar_tab: SidebarTab::TagTree,
        notes: HashMap::new(),
        untagged_notes: vec![],
        current_note: None,
        note_text: None,
        note_dirty: false,
        should_reload_slate: false,
    }
}

struct Model {
    tag_tree: Option<Vec<TagTree>>,
    tag_tree_folds: HashMap<TagId, bool>,
    sidebar_tab: SidebarTab,
    notes: HashMap<NoteId, Note>,
    untagged_notes: Vec<NoteId>,
    current_note: Option<NoteId>,
    note_text: Option<String>,
    note_dirty: bool,
    should_reload_slate: bool, // this is a hack.
}

#[derive(PartialEq)]
enum SidebarTab {
    TagTree,
    Untagged,
}

enum Msg {
    RequestUpdateTagTree,
    UpdateTagTree((Vec<TagTree>, HashMap<NoteId, Note>, Vec<NoteId>)),
    ToggleTag(TagId),
    OpenNote(NoteId),
    NoteBlobLoaded(String),
    RenameNote((Option<NoteId>, String)),
    AddTagToNote((NoteId, TagId)),
    UntagNote((NoteId, TagId)),
    CreateTag(Vec<String>),
    DeleteTag(TagId),
    KeyPressed(web_sys::KeyboardEvent),
    UpdateNoteText(String),
    SaveNote,
    NewNote,
    SidebarShow(SidebarTab),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::RequestUpdateTagTree => {
            orders.skip().perform_cmd(async {
                get_tag_tree().await.map(|t| Msg::UpdateTagTree(t)).ok()
            });
        },
        Msg::UpdateTagTree((tag_tree, notes, untagged_notes)) => {
            model.tag_tree = Some(tag_tree);
            model.notes = notes;
            model.untagged_notes = untagged_notes;
            if model.should_reload_slate {
                orders.send_msg(Msg::OpenNote(model.current_note.unwrap()));
            }
        },
        Msg::ToggleTag(uuid) => {
            *model.tag_tree_folds.entry(uuid).or_insert(false) ^= true;
        },
        Msg::OpenNote(note) => {
            if model.note_dirty {
                orders.send_msg(Msg::SaveNote);
                orders.send_msg(Msg::OpenNote(note));
                model.note_dirty = false;
            } else {
                model.current_note = Some(note);
                if let Some(hash) = model.notes.get(&note).map(|x| x.hash) {
                    orders.perform_cmd(enc!((hash) async move {
                        get_blob(&hex::encode(&hash)).await.map(|b| Msg::NoteBlobLoaded(b)).ok()
                    }));
                } else {
                    model.should_reload_slate = true;
                    orders.send_msg(Msg::RequestUpdateTagTree);
                }
            }
        },
        Msg::NoteBlobLoaded(blob) => {
            model.should_reload_slate = false;
            update_slate(&blob);
        },
        Msg::RenameNote((uuid, name)) => {
            if let Some(uuid) = uuid.or(model.current_note.or(None)) {
                orders.skip().perform_cmd(async move {
                    rename_note(uuid, name).await;
                    Msg::RequestUpdateTagTree
                });
            }
        },
        Msg::AddTagToNote((note, tag)) => {
            orders.skip().perform_cmd(async move {
                add_tag_to_note(note, tag).await;
                Msg::RequestUpdateTagTree
            });
        }
        Msg::UntagNote((note, tag)) => {
            orders.skip().perform_cmd(async move {
                untag_note(note, tag).await;
                Msg::RequestUpdateTagTree
            });
        }
        Msg::CreateTag(tag_name) => {
            orders.skip().perform_cmd(async move {
                create_tag(tag_name).await;
                Msg::RequestUpdateTagTree
            });
        }
        Msg::DeleteTag(tag_id) => {
            orders.skip().perform_cmd(async move {
                delete_tag(tag_id).await;
                Msg::RequestUpdateTagTree
            });
        }
        Msg::SaveNote => {
            match (model.current_note, model.note_text.as_ref()) {
                (Some(uuid), Some(text)) => {
                    orders.perform_cmd(enc!((uuid, text) async move {
                        update_note_text(uuid, text.to_string()).await;
                        Msg::RequestUpdateTagTree
                    }));
                },
                _ => {},
            }
        },
        Msg::NewNote => {
            orders.perform_cmd(async {
                new_note().await.map(|n| Msg::OpenNote(n)).ok()
            });
        },
        Msg::SidebarShow(tab) => {
            model.sidebar_tab = tab;
        },
        Msg::UpdateNoteText(text) => {
            model.note_text = Some(text);
            model.note_dirty = true;
        },
        Msg::KeyPressed(event) => {
            orders.skip();
            match (event.ctrl_key(), event.key().as_str()) {
                (true, "n") => {
                    orders.send_msg(Msg::NewNote);
                    event.prevent_default();
                },
                (true, "s") => {
                    orders.send_msg(Msg::SaveNote);
                    event.prevent_default();
                },
                _ => {},
            }
        },
    }
}

async fn get_tag_tree() -> Result<(Vec<TagTree>, HashMap<NoteId, Note>, Vec<NoteId>), ()> {
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

    let bytes = Request::new("/api/notes/untagged")
        .method(Method::Get)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?
        .bytes().map_err(|e| { log!(e) }).await?;
    let untagged_notes = serde_json::from_slice(&bytes[..]).map_err(|e| { log!(e) })?;

    Ok((tree, notes, untagged_notes))
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

async fn rename_note(uuid: NoteId, name: String) -> Result<(), ()> {
    Request::new(format!("/api/note/{}/rename", uuid))
        .method(Method::Post)
        .body(name.into())
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?;
    Ok(())
}

async fn add_tag_to_note(note: NoteId, tag: TagId) -> Result<(), ()> {
    Request::new(format!("/api/note/{}/tag/{}", note, tag))
        .method(Method::Post)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?;
    Ok(())
}

async fn untag_note(note: NoteId, tag: TagId) -> Result<(), ()> {
    Request::new(format!("/api/note/{}/tag/{}", note, tag))
        .method(Method::Delete)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?;
    Ok(())
}

async fn create_tag(tag_name: Vec<String>) -> Result<(), ()> {
    let bytes = Request::new("/api/tag/new")
        .method(Method::Post)
        .body(serde_json::to_string(&tag_name).unwrap().into())
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?
        .bytes().map_err(|e| { log!(e) }).await?;
    Ok(())
}

async fn delete_tag(tag: TagId) -> Result<(), ()> {
    Request::new(format!("/api/tag/{}", tag))
        .method(Method::Delete)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?;
    Ok(())
}

async fn new_note() -> Result<NoteId, ()> {
    let bytes = Request::new("/api/note/new")
        .method(Method::Post)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?
        .bytes().map_err(|e| { log!(e) }).await?;
    let uuid = serde_json::from_slice(&bytes[..]).map_err(|e| { log!(e) })?;
    log!(uuid);
    Ok(uuid)
}

async fn update_note_text(uuid: NoteId, text: String) -> Result<(), ()> {
    Request::new(format!("/api/note/{}", uuid))
        .method(Method::Post)
        .text(text)
        .fetch()
        .await.map_err(|e| { log!(e); })?
        .check_status().map_err(|e| { log!(e); })?;
    Ok(())
}

fn view(model: &Model) -> Node<Msg> {
    div![
        C!["flex"],
        div![
            id!["left-sidebar"],
            C!["h-screen", "overflow-y-auto", "top-0", "sticky", "p-4", "bg-gray-500"],
            div![
                button![C!["w-1/2", "border", "border-black"], "tree", ev(Ev::Click, |_| Msg::SidebarShow(SidebarTab::TagTree))],
                button![C!["w-1/2", "border", "border-black"], "untagged", ev(Ev::Click, |_| Msg::SidebarShow(SidebarTab::Untagged))],
            ],
            IF![
                model.tag_tree.is_some() && model.sidebar_tab == SidebarTab::TagTree =>
                div![
                    tag_tree_view(model.tag_tree.as_ref().unwrap(), &model.tag_tree_folds, &model.notes, &model.current_note),
                    input![
                        C!["w-full"],
                        attrs!{
                            At::Type => "text",
                            At::Placeholder => "Create new tag",
                        },
                        keyboard_ev(Ev::KeyDown, enc!(() move |event| {
                            if event.key() == "Enter" {
                                let target = event.current_target().unwrap();
                                let input_elem = to_input(&target);
                                let tag_name = input_elem.value().split(">").map(|x| x.to_string()).collect();
                                input_elem.set_value("");

                                Some(Msg::CreateTag(tag_name))
                            } else {
                                None
                            }
                        })),
                    ]
                ]
            ],
            IF![
                model.sidebar_tab == SidebarTab::Untagged =>
                div![untagged_list_view(&model)]
            ],
        ],
        div![
            C!["h-screen", "flex", "flex-col", "flex-grow", "p-4", "bg-gray-100"],
            div![IF![model.current_note.is_some() => note_title_view(&model)]],
            div![
                C!["flex-grow", "overflow-y-auto"],
                id!["editor"],
            ],
        ],
        div![
            id!["right-sidebar"],
            C!["h-screen", "overflow-y-auto", "top-0", "sticky", "p-4", "bg-gray-500"],
            IF![model.current_note.is_some() && model.tag_tree.is_some() => div![
                div![C!["text-2xl"], "tags"],
                get_tags_for_note(&model.tag_tree.as_ref().unwrap(), &model.current_note.unwrap()).iter().map(| tag | {
                    div![
                        C!["tagbubble"],
                        div![
                            C!["tagbubble-inner"],
                            get_tag_name(&model.tag_tree.as_ref().unwrap(), &tag).unwrap().iter().map(|part| div![part]),
                        ],
                        button![
                            C!["ml-auto"],
                            "[-]",
                            ev(Ev::Click, enc!((model.current_note => note, tag) move |_event| {
                                if let Some(note) = note {
                                    Some(Msg::UntagNote((note, tag)))
                                } else {
                                    None
                                }
                            })),
                        ],
                    ]
                }),
                input![
                    C!["w-full"],
                    attrs!{
                        At::Type => "text",
                        At::Placeholder => "Add tag to note",
                    },
                    keyboard_ev(Ev::KeyDown, enc!((
                        model.current_note => note,
                        model.tag_tree => tag_tree,
                    ) move |event| {
                        if event.key() == "Enter" {
                            let target = event.current_target().unwrap();
                            let input_elem = to_input(&target);
                            let tag = get_tag_by_full_name(&tag_tree.unwrap(), input_elem.value().split(">").collect());
                            input_elem.set_value("");

                            if let Some(note) = note {
                                if let Some(tag) = tag {
                                    return Some(Msg::AddTagToNote((note, tag)))
                                }
                            }
                        }
                        None
                    })),
                ]
            ]]
        ],
    ]
}

fn tag_tree_view(tag_tree: &Vec<TagTree>, tag_tree_folds: &HashMap<TagId, bool>, notes: &HashMap<NoteId, Note>, current_note: &Option<NoteId>) -> Node<Msg> {
    ul![
        {
            let mut tag_tree = tag_tree.clone();
            tag_tree.sort_by(|a, b| a.name.cmp(&b.name));
            
            tag_tree.iter().map(|tag| {
                li![
                    IF![!tag_tree_folds.get(&tag.id).unwrap_or(&false) => C!["tree-closed"]],
                    div![
                        C!["flex", "tagtree-row"],
                        button![
                            C!["focus:outline-none"],
                            &tag.name,
                            ev(Ev::Click, enc!((&tag.id => id) move |_| Msg::ToggleTag(id))),
                            ev(Ev::DblClick, enc!((&tag.id => id) move |_| log!("dblclick", id))),
                        ],
                        button![
                            C!["focus:outline-none", "ml-auto", "hidden"],
                            "[-]",
                            ev(Ev::Click, enc!((&tag.id => id) move |_| Msg::DeleteTag(id))),
                        ]
                    ],
                    tag_tree_view(&tag.children, tag_tree_folds, notes, current_note),
                    ul![
                        {
                            let mut tag_notes = tag.notes.clone();
                            tag_notes.sort_by(|a, b| {
                                let a = notes.get(a).unwrap().name.clone();
                                let b = notes.get(b).unwrap().name.clone();
                                a.cmp(&b)
                            });
                            tag_notes.iter().map(|uuid| { note_item_view(*uuid, notes.get(&uuid).unwrap(), current_note) }).collect::<Vec<Node<Msg>>>()
                        },
                    ],
                ]
            }).collect::<Vec<Node<Msg>>>()
        }
    ]
}

fn untagged_list_view(model: &Model) -> Node<Msg> {
    let mut notes = model.untagged_notes.clone();
    notes.sort_by(|a, b| {
        let a = model.notes.get(a).unwrap().name.clone();
        let b = model.notes.get(b).unwrap().name.clone();
        a.cmp(&b)
    });
    ul![
      notes.iter().map(|uuid| { note_item_view(*uuid, model.notes.get(&uuid).unwrap(), &model.current_note) }).collect::<Vec<Node<Msg>>>()
    ]
}

fn note_item_view(uuid: NoteId, note: &Note, current_note: &Option<NoteId>) -> Node<Msg> {
    li![
        C!["note"],
        button![
            C!["focus:outline-none", IF![Some(uuid) == *current_note => "font-bold"]],
            note.name.as_str(),
            ev(Ev::Click, enc!((uuid) move |_| Msg::OpenNote(uuid))),
        ],
    ]
}

fn note_title_view(model: &Model) -> Node<Msg> {
    input![
        C!["bg-transparent", "text-3xl", "mb-4", "w-full", "focus:outline-none"],
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
    ]
}

#[wasm_bindgen]
pub fn start() -> Box<[JsValue]> {
    let app = App::start("app", init, update, view);

    create_closures_for_js(&app)
}

fn create_closures_for_js(app: &App<Msg, Model, Node<Msg>>) -> Box<[JsValue]> {
    let update_content = wrap_in_permanent_closure(enc!((app) move |content| {
        app.update(Msg::UpdateNoteText(content))
    }));

    vec![update_content].into_boxed_slice()
}

fn wrap_in_permanent_closure<T>(f: impl FnMut(T) + 'static) -> JsValue
where
    T: wasm_bindgen::convert::FromWasmAbi + 'static,
{
    let closure = Closure::new(f);
    let closure_as_js_value = closure.as_ref().clone();
    closure.forget();
    closure_as_js_value
}

#[wasm_bindgen]
extern "C" {
    fn start_slate();
    #[wasm_bindgen(js_namespace = window)]
    fn update_slate(_: &str);
}
