use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Element, Event, HtmlElement, HtmlInputElement, Request, RequestInit, RequestMode, Response, console};
use js_sys::{Function, Uint8Array};

use libcommonplace_types::{TagTree, Note};

async fn api_get<'a, T: ?Sized>(path: &str) -> Result<T, JsValue>
where
    for<'de> T: serde::de::Deserialize<'de> + 'a
{
    let window = web_sys::window().unwrap();
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::SameOrigin);
    let request = Request::new_with_str_and_init(path, &opts)?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();
    let json = JsFuture::from(resp.json()?).await?;
    json.into_serde().map_err(|_| JsValue::NULL)
}

async fn blob_get(hash: &str) -> Result<Uint8Array, JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::SameOrigin);
    let request = Request::new_with_str_and_init(&format!("/api/blob/{}", hash), &opts)?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();
    Ok(Uint8Array::new(&JsFuture::from(resp.array_buffer()?).await?))
}

fn render_sidebar(tag_tree: Vec<TagTree>) -> Result<Element, JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let e = document.create_element("ul")?;

    for tag in tag_tree {
        let list_item = document.create_element("li")?;
        let list_inner = document.create_element("div")?;
        list_inner.set_inner_html(&tag.name);
        list_inner.class_list().add_1("tag")?;
        list_inner.set_attribute("onclick", "tag_click(event)")?;
        list_inner.set_attribute("data-uuid", &format!("{}", tag.id))?;
        list_item.append_child(&list_inner)?;
        list_item.append_child(&render_sidebar(tag.children)?.into())?;
        let note_list = document.create_element("ul")?;
        for note in tag.notes {
            let list_item = document.create_element("li")?;
            let list_inner = document.create_element("div")?;
            list_inner.set_inner_html(&note.name);
            list_inner.class_list().add_1("note")?;
            list_inner.set_attribute("onclick", "note_click(event)")?;
            list_inner.set_attribute("data-uuid", &format!("{}", note.id))?;
            list_item.append_child(&list_inner)?;
            note_list.append_child(&list_item)?;
        }
        list_item.append_child(&note_list)?;
        e.append_child(&list_item)?;
    }

    Ok(e)
}

async fn load_note(uuid: &str) {
    let note: Note = api_get(&format!("/api/note/{}", uuid)).await.unwrap();
    let contents = blob_get(&hex::encode(note.hash)).await.unwrap().to_vec();
    let contents = std::str::from_utf8(&contents).unwrap();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let update_slate = window.get("update_slate").unwrap().dyn_into::<Function>().unwrap();
    update_slate.call1(&window, &contents.into());
    let title = document.get_element_by_id("title").unwrap().dyn_into::<HtmlInputElement>().unwrap();
    title.set_value(&note.name);
    title.set_attribute("onblur", &format!("rename_note('{}', event.target.value)", uuid));
}

#[wasm_bindgen]
pub async fn rename_note(uuid: String, name: String) {
    let window = web_sys::window().unwrap();
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::SameOrigin);
    opts.body(Some(&name.into()));
    let request = Request::new_with_str_and_init(&format!("/api/note/{}/rename", uuid), &opts).unwrap();
    JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
}

#[wasm_bindgen]
pub fn tag_click(e: Event) {
    let elem = e.target().unwrap().dyn_into::<Element>().unwrap().parent_element().unwrap();
    elem.class_list().toggle(&"tree-closed").unwrap();
}

#[wasm_bindgen]
pub async fn note_click(e: Event) {
    let uuid = e.target().unwrap().dyn_into::<Element>().unwrap().get_attribute("data-uuid").unwrap();
    load_note(&uuid).await;
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    console::log_1(&"loaded!".into());

    let tagtree: Vec<TagTree> = api_get("/api/showtree").await?;
    document.get_element_by_id("sidebar").unwrap().set_inner_html(&render_sidebar(tagtree)?.outer_html());

    Ok(())
}
