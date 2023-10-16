use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::StateDirTrait;

use crate::node::get_node_name;
use crate::node::util::{delete_all_nodes, delete_node, delete_selected_nodes, get_all_node_names};

use crate::util::local_cmd;
use crate::{docs, fmt_ok, CommandGlobalOpts};

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
    Selected(Option<Vec<String>>),
    Single(Option<String>),
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> miette::Result<()> {
    let all_nodes = opts.state.nodes.list_items_names()?;

    let delete_mode = if cmd.all {
        DeleteMode::All
    } else if cmd.node_name.is_none()
        && !all_nodes.is_empty()
        && opts.terminal.can_ask_for_user_input()
    {
        DeleteMode::Selected(opts.terminal.select_multiple(
            "Select one or more nodes that you want to delete".to_string(),
            all_nodes,
        ))
    } else {
        DeleteMode::Single(cmd.node_name)
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
        DeleteMode::Single(cmd_node_name) => {
            if opts.terminal.confirmed_with_flag_or_prompt(
                cmd.yes,
                "Are you sure you want to delete this node?",
            )? {
                let node_name = get_node_name(&opts.state, &cmd_node_name);
                delete_node(&opts, &node_name, cmd.force)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Node with name '{}' has been deleted", &node_name))
                    .machine(&node_name)
                    .json(serde_json::json!({ "node": { "name": &node_name } }))
                    .write_line()?;
            }
        }
        DeleteMode::Selected(option_selected_node_names) => {
            let selected_node_names = option_selected_node_names.unwrap();
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
                selected_node_names
                    .iter()
                    .map(|name| (name, opts.state.nodes.delete_sigkill(name, cmd.force)))
                    .for_each(|(name, res)| {
                        if res.is_ok() {
                            opts.terminal
                                .clone()
                                .stdout()
                                .plain(format!("✅ Deleted Node: '{}'", name))
                                .machine(name)
                                .write_line()
                                .unwrap();
                        } else {
                            opts.terminal
                                .clone()
                                .stdout()
                                .plain(format!(
                                    "⚠️ Failed to delete Node: '{}', Error: '{}'",
                                    name,
                                    res.as_ref().unwrap_err()
                                ))
                                .write_line()
                                .unwrap();
                        }
                    });
            }
        }
    };
    Ok(())
}
