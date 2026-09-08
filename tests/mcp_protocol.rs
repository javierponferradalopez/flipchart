use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Session {
    process: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
}

impl Session {
    fn open() -> Self {
        let mut process = Command::new(env!("CARGO_BIN_EXE_flipchart"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the flipchart binary starts");
        let input = process.stdin.take().unwrap();
        let output = BufReader::new(process.stdout.take().unwrap());
        let mut session = Self {
            process,
            input,
            output,
            next_id: 1,
        };
        session.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }),
        );
        session.notify("notifications/initialized");
        session
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.input, "{request}").unwrap();
        self.input.flush().unwrap();
        loop {
            let mut line = String::new();
            self.output
                .read_line(&mut line)
                .expect("the server answers");
            let message: Value = serde_json::from_str(line.trim()).expect("readable JSON-RPC");
            if message.get("id") == Some(&json!(id)) {
                return message["result"].clone();
            }
        }
    }

    fn notify(&mut self, method: &str) {
        let notification = json!({ "jsonrpc": "2.0", "method": method });
        writeln!(self.input, "{notification}").unwrap();
        self.input.flush().unwrap();
    }

    fn tools(&mut self) -> Vec<Value> {
        self.request("tools/list", json!({}))["tools"]
            .as_array()
            .expect("tools/list brings a list")
            .clone()
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn tool<'a>(tools: &'a [Value], name: &str) -> &'a Value {
    tools
        .iter()
        .find(|t| t["name"] == json!(name))
        .unwrap_or_else(|| panic!("the {name} tool is registered"))
}

fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().expect("some text")
}

#[test]
fn the_server_exposes_show_clear_and_marks_and_nothing_else() {
    let mut session = Session::open();

    let mut names: Vec<String> = session
        .tools()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();

    assert_eq!(names, ["clear", "marks", "show"]);
}

#[test]
fn show_carries_the_literal_of_its_description() {
    let mut session = Session::open();

    let show = tool(&session.tools(), "show").clone();

    assert_eq!(
        show["description"],
        json!(
            "Show a diagram on the ephemeral flipchart window, as a named view. Takes Mermaid source.\n\n\
             Any id used in a relationship must carry a label or a body when another id in the same \
             diagram does; a bare id alongside a labelled one is rejected.\n\n\
             Showing an existing view id replaces it and brings it to the front; several named views \
             coexist. The flipchart dies with the session."
        )
    );
}

#[test]
fn clear_carries_the_literal_of_its_description() {
    let mut session = Session::open();

    let clear = tool(&session.tools(), "clear").clone();

    assert_eq!(
        clear["description"],
        json!("Remove one view from the flipchart, or all of them. Does not close the window.")
    );
}

#[test]
fn marks_carries_the_literal_of_its_description() {
    let mut session = Session::open();

    let marks = tool(&session.tools(), "marks").clone();

    assert_eq!(
        marks["description"],
        json!(
            "Deliver the marks the user drew over the views: one image per view - the whole \
             sheet with the ink baked in, never the ink alone. No arguments, never an error; \
             one line means nothing new. Undelivered marks survive a replace of their view and \
             arrive annotated; delivered ones die at your next show over it - the redraw is \
             the reply."
        )
    );
}

#[test]
fn marks_asks_for_nothing() {
    let mut session = Session::open();

    let schema = tool(&session.tools(), "marks")["inputSchema"].clone();

    assert!(
        schema["required"]
            .as_array()
            .map(|r| r.is_empty())
            .unwrap_or(true)
    );
}

#[test]
fn an_empty_inbox_answers_as_one_line_of_text_and_nothing_else() {
    let mut session = Session::open();
    session.call(
        "show",
        json!({ "view_id": "current", "diagram": "flowchart LR\n  A[One] --> B[Two]\n" }),
    );

    let result = session.call("marks", json!({}));

    assert_eq!(result["isError"], json!(false));
    let content = result["content"].as_array().expect("a content list");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], json!("text"));
    assert_eq!(content[0]["text"], json!("No marks waiting."));
}

#[test]
fn show_asks_for_view_id_and_diagram_and_both_are_required() {
    let mut session = Session::open();

    let schema = tool(&session.tools(), "show")["inputSchema"].clone();

    let mut required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    required.sort();
    assert_eq!(required, ["diagram", "view_id"]);
}

#[test]
fn the_view_id_of_show_is_described_with_its_example() {
    let mut session = Session::open();

    let schema = tool(&session.tools(), "show")["inputSchema"].clone();

    assert_eq!(
        schema["properties"]["view_id"]["description"],
        json!(
            "Short human-readable name, shown to the user above the diagram - e.g. \
             \"Current dependencies\", not \"v1\". Reusing a name replaces that view."
        )
    );
}

#[test]
fn clear_asks_for_nothing() {
    let mut session = Session::open();

    let schema = tool(&session.tools(), "clear")["inputSchema"].clone();

    assert!(
        schema["required"]
            .as_array()
            .map(|r| r.is_empty())
            .unwrap_or(true)
    );
}

#[test]
fn show_returns_the_acknowledgement_with_the_recount_and_the_live_views() {
    let mut session = Session::open();

    let result = session.call(
        "show",
        json!({ "view_id": "current", "diagram": "flowchart LR\n  A[One] --> B[Two]\n" }),
    );

    assert_eq!(
        text(&result),
        "Shown as view \"current\" (2 nodes, 1 edge). Views on the flipchart: current."
    );
}

#[test]
fn a_note_travels_with_the_drawn_view_and_not_as_an_error() {
    let mut session = Session::open();

    let result = session.call(
        "show",
        json!({
            "view_id": "current",
            "diagram": "flowchart TB\n  classDef danger fill:#f00\n  A[One] --> B[Two]\n"
        }),
    );

    assert_eq!(result["isError"], json!(false));
}

#[test]
fn the_notes_arrive_after_the_acknowledgement_and_accumulate() {
    let mut session = Session::open();

    let result = session.call(
        "show",
        json!({
            "view_id": "current",
            "diagram": "flowchart TB\n  classDef danger fill:#f00\n  A[One] --> B[Two]\n"
        }),
    );

    assert_eq!(
        text(&result),
        "Shown as view \"current\" (2 nodes, 1 edge). Views on the flipchart: current.\n\
         Note: style directives (classDef, class, style, linkStyle) and click links were \
         dropped — the flipchart decides how views look. The view was drawn.\n\
         Note: the flipchart lays diagrams out left to right; the direction in your source \
         was ignored. The view was drawn."
    );
}

#[test]
fn the_flipchart_state_survives_between_calls() {
    let mut session = Session::open();
    let diagram = json!("flowchart TD\n  A[One] --> B[Two]\n");
    session.call("show", json!({ "view_id": "current", "diagram": diagram }));
    session.call("show", json!({ "view_id": "proposed", "diagram": diagram }));

    let result = session.call("clear", json!({ "view_id": "current" }));

    assert_eq!(
        text(&result),
        "Cleared view \"current\". Views on the flipchart: proposed."
    );
}

#[test]
fn an_invalid_input_comes_back_marked_as_a_tool_error() {
    let mut session = Session::open();

    let result = session.call(
        "show",
        json!({ "view_id": "", "diagram": "flowchart TD\n A\n" }),
    );

    assert_eq!(result["isError"], json!(true));
}

#[test]
fn a_node_the_agent_did_not_declare_comes_back_inside_the_result_and_not_as_a_transport_error() {
    let mut session = Session::open();

    let result = session.call(
        "show",
        json!({ "view_id": "proposed", "diagram": "flowchart TD\n  API[API Layer] --> Db\n" }),
    );

    assert_eq!(result["isError"], json!(true));
}

#[test]
fn the_rejection_says_nothing_was_drawn_and_the_view_is_as_it_was() {
    let mut session = Session::open();

    let result = session.call(
        "show",
        json!({ "view_id": "proposed", "diagram": "flowchart TD\n  API[API Layer] --> Db\n" }),
    );

    assert_eq!(
        text(&result),
        "Rejected: nothing was drawn; view \"proposed\" is unchanged.\n\
         1 node appears in the drawing that you did not declare.\n  \
         \"Db\"  line 2  — only used in a relation\n\
         Declare every node you name, and rewrite any line the renderer turned into one."
    );
}

#[test]
fn a_rejection_does_not_touch_the_view_already_on_screen() {
    let mut session = Session::open();
    session.call(
        "show",
        json!({ "view_id": "proposed", "diagram": "flowchart TD\n  A[One] --> B[Two]\n" }),
    );

    session.call(
        "show",
        json!({ "view_id": "proposed", "diagram": "flowchart TD\n  API[API Layer] --> Db\n" }),
    );

    assert_eq!(
        text(&session.call("clear", json!({ "view_id": "proposed" }))),
        "Cleared view \"proposed\". No views."
    );
}
