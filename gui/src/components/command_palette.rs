// Command palette component (VSCode style)
#![allow(non_snake_case)]
use dioxus::prelude::*;
use fuzzy_matcher::FuzzyMatcher; // Import FuzzyMatcher
use fuzzy_matcher::skim::SkimMatcherV2; // Import SkimMatcherV2

use crate::state::app_state::AppState; // To control visibility and dispatch actions

// --- Command Structures ---

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    LoadCsv { path: Option<String> },
    Configure,
    Exit,
    AddIndicator { indicator_type: String },
    RemoveIndicator { name: String },
    SaveProject { path: Option<String> },
    LoadProject { path: Option<String> },
}

#[derive(Clone, Debug)] // Added Debug for easier inspection
pub struct CommandDefinition {
    pub id: usize, // Unique ID for keying and selection
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,
    pub action: Command,
}

impl CommandDefinition {
    fn new(id: usize, name: &str, description: &str, action: Command) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            shortcut: None,
            action,
        }
    }
}

// --- End Command Structures ---

#[component]
pub fn CommandPalette(cx: Scope) -> Element {
    let app_state = use_shared_state::<AppState>(cx).unwrap();

    let all_commands = use_ref(cx, || {
        vec![
            CommandDefinition::new(0, "Load CSV Data...", "Import market data from a CSV file", Command::LoadCsv { path: None }),
            CommandDefinition::new(1, "Add Indicator: SMA", "Add Simple Moving Average indicator", Command::AddIndicator { indicator_type: "SMA".to_string() }),
            CommandDefinition::new(2, "Add Indicator: EMA", "Add Exponential Moving Average indicator", Command::AddIndicator { indicator_type: "EMA".to_string() }),
            CommandDefinition::new(3, "Add Indicator: RSI", "Add Relative Strength Index indicator", Command::AddIndicator { indicator_type: "RSI".to_string() }),
            CommandDefinition::new(4, "Exit Application", "Close Home Trader", Command::Exit),
            // More commands...
        ]
    });

    let filter_text = use_state(cx, String::new);
    let selected_index = use_state(cx, || 0usize);
    let matcher = use_ref(cx, SkimMatcherV2::default); // Fuzzy matcher instance

    let filtered_commands = use_memo(cx, (filter_text, all_commands), move |(filter, cmds_ref)| {
        let cmds = cmds_ref.read();
        if filter.is_empty() {
            return cmds.clone();
        }
        let mut scored_commands: Vec<(i64, CommandDefinition)> = cmds
            .iter()
            .filter_map(|cmd| {
                matcher.read().fuzzy_match(&cmd.name, filter)
                    .map(|score| (score, cmd.clone()))
            })
            .collect();

        scored_commands.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by score descending
        scored_commands.into_iter().map(|(_, cmd)| cmd).collect::<Vec<_>>()
    });

    // Reset selected index when filter changes
    use_effect(cx, filtered_commands, |cmds| {
        selected_index.set(0);
        async move { cmds } // use_effect expects a future
    });

    if !app_state.read().command_palette_visible {
        return None;
    }

    let handle_keydown = move |evt: KeyboardEvent| {
        let current_filtered_commands = filtered_commands.read();
        if current_filtered_commands.is_empty() {
            return;
        }

        match evt.key() {
            Key::ArrowDown => {
                selected_index.set((selected_index.get() + 1) % current_filtered_commands.len());
            }
            Key::ArrowUp => {
                selected_index.set(
                    (selected_index.get() + current_filtered_commands.len() - 1) % current_filtered_commands.len(),
                );
            }
            Key::Enter => {
                if let Some(cmd_def) = current_filtered_commands.get(*selected_index.get()) {
                    execute_command(cmd_def.action.clone(), app_state, filter_text);
                }
            }
            Key::Escape => {
                 app_state.write().command_palette_visible = false;
                 filter_text.set(String::new()); // Reset filter
            }
            _ => {}
        }
    };

    // Separate function to handle command execution logic
    let execute_command = |command: Command, app_state: &UseSharedState<AppState>, filter_text_state: &UseState<String>| {
        match command {
            Command::LoadCsv { path } => {
                // For now, path is None. Later, this could open a file dialog or take input.
                // If path is Some, use it.
                let file_to_load = path.unwrap_or_else(|| "path/to/default.csv".to_string()); // Placeholder
                tracing::info!("[COMMAND ACTION] Load CSV: {}", file_to_load);
                // TODO: Call engine_client.load_csv(file_to_load, "SYMBOL")
                // For now, we can simulate adding some data to AppState if needed for UI testing
            }
            Command::AddIndicator { indicator_type } => {
                tracing::info!("[COMMAND ACTION] Add Indicator: {}", indicator_type);
                // TODO: Update AppState with the new indicator or call engine_client
            }
            Command::Exit => {
                tracing::info!("[COMMAND ACTION] Exit Application");
                // Attempt to close the window. This is platform-specific.
                // For Dioxus desktop, you might use the window handle.
                // This is a simplified attempt; real exit might need more cleanup.
                dioxus_desktop::use_window(cx).close();
            }
            Command::Configure => {
                tracing::info!("[COMMAND ACTION] Configure (Not implemented yet)");
            }
            Command::RemoveIndicator { name } => {
                tracing::info!("[COMMAND ACTION] Remove Indicator: {} (Not implemented yet)", name);
            }
            Command::SaveProject { path } => {
                let file_to_save = path.unwrap_or_else(|| "project_state.json".to_string());
                tracing::info!("[COMMAND ACTION] Save Project to {}: (Not implemented yet)", file_to_save);
            }
            Command::LoadProject { path } => {
                let file_to_load = path.unwrap_or_else(|| "project_state.json".to_string());
                tracing::info!("[COMMAND ACTION] Load Project from {}: (Not implemented yet)", file_to_load);
            }
        }
        app_state.write().command_palette_visible = false;
        filter_text_state.set(String::new()); // Reset filter
    };


    cx.render(rsx! {
        div {
            class: "command-palette",
            style: "position: fixed; top: 10%; left: 50%; transform: translateX(-50%); background-color: #333; color: #eee; border: 1px solid #555; padding: 15px; z-index: 1000; width: 600px; border-radius: 8px; box-shadow: 0 5px 15px rgba(0,0,0,0.5);",
            onkeydown: handle_keydown,
            input {
                id: "command-palette-input", // Added id for potential focus
                r#type: "text",
                value: "{filter_text}",
                placeholder: "Type a command...",
                autofocus: true, // Focus input on render
                style: "width: calc(100% - 20px); padding: 10px; margin-bottom: 10px; background-color: #444; color: #eee; border: 1px solid #666; border-radius: 4px;",
                oninput: move |evt| {
                    filter_text.set(evt.value.clone());
                },
            }
            ul {
                style: "list-style: none; padding: 0; margin: 0; max-height: 300px; overflow-y: auto;",
                if filtered_commands.read().is_empty() {
                    rsx! {
                         li { style: "padding: 8px; color: #888;", "No commands match your search."}
                    }
                } else {
                    filtered_commands.read().iter().enumerate().map(|(idx, cmd_def)| {
                        let bg_color = if idx == *selected_index.get() { "#555" } else { "transparent" };
                        let current_cmd_def = cmd_def.clone(); // Clone for the closure
                        rsx! {
                            li {
                                key: "{current_cmd_def.id}", // Use unique ID for key
                                style: "padding: 10px 12px; border-bottom: 1px solid #444; cursor: pointer; background-color: {bg_color}; border-radius: 3px;",
                                onclick: move |_| {
                                    execute_command(current_cmd_def.action.clone(), app_state, filter_text);
                                },
                                onmouseenter: move |_| {
                                    selected_index.set(idx);
                                },
                                div { style: "font-weight: bold;", "{cmd_def.name}" }
                                div { style: "font-size: 0.9em; color: #aaa;", "{cmd_def.description}" }
                            }
                        }
                    })
                }
            }
        }
    })
}
