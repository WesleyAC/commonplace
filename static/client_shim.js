import init, { tag_click, note_click } from './wasm/commonplace_gui_client.js';

async function run() {
	await init();

	window.tag_click = tag_click;
	window.note_click = note_click;
}

run();
