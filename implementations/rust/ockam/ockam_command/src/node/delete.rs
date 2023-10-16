use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::StateDirTrait;

use crate::node::get_default_node_name;
use crate::node::util::{delete_all_nodes, delete_node};

use crate::util::local_cmd;
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete nodes
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the node to be deleted
    #[arg(group = "nodes")]
    node_name: Option<String>,

    /// Terminate all node processes and delete all node configurations
    #[arg(long, short, group = "nodes")]
    all: bool,

    /// Terminate node process(es) immediately (uses SIGKILL instead of SIGTERM)
    #[arg(display_order = 901, long, short)]
    force: bool,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

enum DeleteMode {
    All,
    Selected(Vec<String>),
    Single(String),
    Default,
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
    let all_nodes = opts.state.nodes.list_items_names()?;

    let delete_mode = if cmd.all {
        DeleteMode::All
    } else if cmd.node_name.is_some() {
        DeleteMode::Single(cmd.node_name.unwrap())
    } else if !all_nodes.is_empty() && opts.terminal.can_ask_for_user_input() {
        DeleteMode::Selected(
            opts.terminal
                .select_multiple(
                    "Select one or more nodes that you want to delete".to_string(),
                    all_nodes,
                )
                .unwrap(),
        )
    } else {
        DeleteMode::Default
    };

    match delete_mode {
        DeleteMode::All => {
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                "Are you sure you want to delete all nodes?",
            )? {
                delete_all_nodes(&opts, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("All nodes have been deleted"))
                    .write_line()?;
            }
        }
        DeleteMode::Single(node_name) => {
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                "Are you sure you want to delete this node?",
            )? {
                delete_node(&opts, &node_name, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Node with name '{}' has been deleted", &node_name))
                    .machine(&node_name)
                    .json(serde_json::json!({ "node": { "name": &node_name } }))
                    .write_line()?;
            }
        }
        DeleteMode::Selected(selected_node_names) => {
            if selected_node_names.is_empty() {
                opts.terminal
                    .stdout()
                    .plain("No nodes selected for deletion")
                    .write_line()?;
                return Ok(());
            }

            if opts
                .terminal
                .confirm_interactively(format!(
                    "Would you like to delete these items : {:?}?",
                    selected_node_names
                ))
                .unwrap_or(false)
            {
                let output = selected_node_names
                    .iter()
                    .map(|name| (name, opts.state.nodes.delete_sigkill(name, cmd.force)))
                    .map(|(name, res)| {
                        if res.is_ok() {
                            fmt_ok!("Deleted Node: '{}'\n", name)
                        } else {
                            fmt_warn!(
                                "Failed to delete Node: '{}', Error: '{}'\n",
                                name,
                                res.as_ref().unwrap_err()
                            )
                        }
                    })
                    .collect::<String>();

                opts.terminal.stdout().plain(output).write_line()?;
            }
        }
        DeleteMode::Default => {
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                "Are you sure you want to delete the default node?",
            )? {
                let node_name = get_default_node_name(&opts.state);
                delete_node(&opts, &node_name, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Node with name '{}' has been deleted", &node_name))
                    .machine(&node_name)
                    .json(serde_json::json!({ "node": { "name": &node_name } }))
                    .write_line()?;
            }
        }
    };
    Ok(())
}
