const App = () => {
	const editor = React.useMemo(() => {
		return SlateHistory.withHistory(SlateReact.withReact(Slate.createEditor()))
	}, []);

	const [value, setValue] = React.useState([]);

	React.useEffect(() => {
		window.update_slate = (text) => {
			setValue([{
				type: 'paragraph',
				children: [{
					text: text
				}]
			}]);
		};
	}, [])


	return React.createElement(SlateReact.Slate, {
		editor: editor,
		value: value,
		onChange: newValue => setValue(newValue)
	}, React.createElement(SlateReact.Editable, null));
};

function start_slate() {
	ReactDOM.render(React.createElement(App), document.getElementById("editor"));
}
