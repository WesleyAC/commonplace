use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Element, Event, HtmlElement, Request, RequestInit, RequestMode, Response, console};
use js_sys::Function;

use libcommonplace_types::TagTree;

async fn api_get<'a, T: ?Sized>(path: &str) -> Result<T, JsValue>
where
    for<'de> T: serde::de::Deserialize<'de> + 'a
{
    let window = web_sys::window().expect("no global `window` exists");
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

fn render_sidebar(tag_tree: Vec<TagTree>) -> Result<Element, JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let e = document.create_element("ul")?;

    for tag in tag_tree {
        let list_item = document.create_element("li")?;
        let list_inner = document.create_element("div")?;
        list_inner.set_inner_html(&tag.name);
        list_inner.class_list().add_1("tag")?;
        list_inner.set_attribute("onclick", "tag_click(event)");
        list_item.append_child(&list_inner)?;
        list_item.append_child(&render_sidebar(tag.children)?.into())?;
        let note_list = document.create_element("ul")?;
        for note in tag.notes {
            let list_item = document.create_element("li")?;
            let list_inner = document.create_element("div")?;
            list_inner.set_inner_html(&note.name);
            list_inner.class_list().add_1("note")?;
            list_item.append_child(&list_inner)?;
            note_list.append_child(&list_item)?;
        }
        list_item.append_child(&note_list)?;
        e.append_child(&list_item)?;
    }

    Ok(e)
}

#[wasm_bindgen]
pub fn tag_click(e: Event) {
    let elem = e.target().unwrap().dyn_into::<Element>().unwrap().parent_element().unwrap();

    let children = elem.query_selector_all(":scope > ul").unwrap();
    js_sys::Array::from(&children).for_each(&mut |c, _, _| {
        let elem = c.dyn_ref::<HtmlElement>().unwrap();
        let current_display = elem.style().get_property_value("display").unwrap();
        elem.style().set_property("display", if current_display == "none" { "block" } else { "none" });
    });
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
