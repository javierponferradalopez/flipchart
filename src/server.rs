use std::sync::Mutex;

use base64::Engine as _;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, ServiceExt, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::signal::unix::{SignalKind, signal};

mod flipchart;
mod lifecycle;

use self::flipchart::Flipchart;
use self::flipchart::MarkedSheet;
use self::lifecycle::the_session_is_over;
use crate::wire::Wire;

#[derive(Deserialize, JsonSchema)]
pub struct ShowParams {
    #[schemars(
        description = "Short human-readable name, shown to the user above the diagram - e.g. \"Current dependencies\", not \"v1\". Reusing a name replaces that view."
    )]
    view_id: String,
    #[schemars(description = "Mermaid source.")]
    diagram: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ClearParams {
    #[schemars(description = "View to remove. Omit to clear the whole flipchart.")]
    view_id: Option<String>,
}

pub struct FlipchartServer {
    flipchart: Mutex<Flipchart>,
}

#[tool_router]
impl FlipchartServer {
    pub fn new(viewer: Wire) -> Self {
        Self {
            flipchart: Mutex::new(Flipchart::new(viewer)),
        }
    }

    #[tool(
        description = "Show a diagram on the ephemeral flipchart window, as a named view. Takes Mermaid source.\n\nAny id used in a relationship must carry a label or a body when another id in the same diagram does; a bare id alongside a labelled one is rejected.\n\nShowing an existing view id replaces it and brings it to the front; several named views coexist. The flipchart dies with the session."
    )]
    async fn show(&self, Parameters(params): Parameters<ShowParams>) -> CallToolResult {
        match self
            .flipchart
            .lock()
            .expect("the flipchart lock is never held across a panic")
            .show(&params.view_id, &params.diagram)
        {
            Ok(acknowledgement) => {
                CallToolResult::success(vec![ContentBlock::text(acknowledgement)])
            }
            Err(rejection) => CallToolResult::error(vec![ContentBlock::text(rejection)]),
        }
    }

    #[tool(
        description = "Deliver the marks the user drew over the views: one image per view - the whole sheet with the ink baked in, never the ink alone. No arguments, never an error; one line means nothing new. Undelivered marks survive a replace of their view and arrive annotated; delivered ones die at your next show over it - the redraw is the reply."
    )]
    async fn marks(&self) -> CallToolResult {
        let delivered = self
            .flipchart
            .lock()
            .expect("the flipchart lock is never held across a panic")
            .marks();
        let mut content = Vec::new();
        for sheet in delivered {
            content.push(ContentBlock::text(the_reply_to(&sheet)));
            content.push(ContentBlock::image(
                base64::engine::general_purpose::STANDARD.encode(sheet.png),
                "image/png",
            ));
        }
        if content.is_empty() {
            content.push(ContentBlock::text("No marks waiting."));
        }
        CallToolResult::success(content)
    }

    #[tool(
        description = "Remove one view from the flipchart, or all of them. Does not close the window."
    )]
    async fn clear(&self, Parameters(params): Parameters<ClearParams>) -> CallToolResult {
        let text = self
            .flipchart
            .lock()
            .expect("the flipchart lock is never held across a panic")
            .clear(params.view_id.as_deref());
        CallToolResult::success(vec![ContentBlock::text(text)])
    }
}

/// The line that introduces a delivered image. Ink folded against a sheet the
/// agent has since replaced says so: the circles are about the old diagram,
/// and the agent must not read them as comments on the new one.
fn the_reply_to(sheet: &MarkedSheet) -> String {
    if sheet.before_the_last_replace {
        format!(
            "Marks on view \"{}\" - drawn over the sheet as it was before the last replace:",
            sheet.view_id
        )
    } else {
        format!("Marks on view \"{}\":", sheet.view_id)
    }
}

#[rmcp::tool_handler]
impl ServerHandler for FlipchartServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("flipchart", env!("CARGO_PKG_VERSION")))
    }
}

pub fn serve(viewer: Wire) -> ! {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("the server thread owns its runtime");
    runtime.block_on(until_the_session_dies(viewer.clone()));
    the_session_is_over(&viewer)
}

/// The two death signals, in the order they arrive: `SIGINT` —the first thing
/// the host sends— and the EOF on stdin that the MCP-over-stdio specification
/// requires us to handle. `SIGINT` is listened for from before `initialize`,
/// because from the moment it is registered it stops killing the process on its
/// own.
async fn until_the_session_dies(viewer: Wire) {
    let mut interrupted =
        signal(SignalKind::interrupt()).expect("the server thread listens for SIGINT");
    tokio::select! {
        _ = interrupted.recv() => {}
        _ = mcp_over_stdio(viewer) => {}
    }
}

async fn mcp_over_stdio(viewer: Wire) {
    let Ok(service) = FlipchartServer::new(viewer)
        .serve(rmcp::transport::stdio())
        .await
    else {
        return;
    };
    let _ = service.waiting().await;
}
