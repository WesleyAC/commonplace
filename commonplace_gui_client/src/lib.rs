use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Element, Request, RequestInit, RequestMode, Response};

use libcommonplace_types::TagTree;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

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

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    
    console_log!("loaded!");

    let tagtree: Vec<TagTree> = api_get("/api/showtree").await?;
    document.get_element_by_id("sidebar").unwrap().set_inner_html(&render_sidebar(tagtree)?.outer_html());

    Ok(())
}
