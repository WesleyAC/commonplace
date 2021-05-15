const serialize = value => {
	return (
		value
		.map(n => Slate.Node.string(n))
		.join('\n')
	)
}

const deserialize = string => {
	return string.split('\n').map(line => {
		return {
			children: [{ text: line }],
		}
	})
}

const App = () => {
	const editor = React.useMemo(() => {
		return SlateHistory.withHistory(SlateReact.withReact(Slate.createEditor()))
	}, []);

	const [value, setValue] = React.useState([]);

	React.useEffect(() => {
		window.update_slate = (text) => {
			setValue(deserialize(text));
		};
	}, [])


	return React.createElement(SlateReact.Slate, {
		editor: editor,
		value: value,
		onChange: newValue => {
			window.update_content(serialize(newValue));
			setValue(newValue);
		}
	}, React.createElement(SlateReact.Editable, null));
};

function start_slate() {
	ReactDOM.render(React.createElement(App), document.getElementById("editor"));
}
