// Command palette component (VSCode style)
#![allow(non_snake_case)]
use dioxus::prelude::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::state::app_state::AppState;
use crate::config::AppConfig; // Import AppConfig
use crate::services::engine_client::EngineClient; // Import EngineClient
use shared::models::{MarketData, Candle, Indicator}; // Import shared models
use serde_json::json; // For indicator parameters

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
    let app_config = use_shared_state::<AppConfig>(cx).unwrap();
    // Clone the engine_client for use in async tasks. The Option<EngineClient> itself is not Clone,
    // but the UseSharedState<Option<EngineClient>> handle is.
    let engine_client_handle = use_shared_state::<Option<EngineClient>>(cx).unwrap();


    let all_commands = use_ref(cx, || {
        vec![
            CommandDefinition::new(0, "Load CSV Data (Sample WINFUT)", "Import WINFUT market data from a sample CSV file", Command::LoadCsv { path: Some("tests/data/sample.csv".to_string()) }),
            CommandDefinition::new(1, "Add Indicator: SMA", "Add Simple Moving Average indicator", Command::AddIndicator { indicator_type: "SMA".to_string() }),
            CommandDefinition::new(2, "Add Indicator: EMA", "Add Exponential Moving Average indicator", Command::AddIndicator { indicator_type: "EMA".to_string() }),
            CommandDefinition::new(3, "Add Indicator: RSI", "Add Relative Strength Index indicator", Command::AddIndicator { indicator_type: "RSI".to_string() }),
            CommandDefinition::new(4, "Exit Application", "Close Home Trader", Command::Exit),
            // More commands...
        ]
    });

    let filter_text = use_state(cx, String::new);
    let selected_index = use_state(cx, || 0usize);
    let matcher = use_ref(cx, SkimMatcherV2::default);

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

        scored_commands.sort_by(|a, b| b.0.cmp(&a.0));
        scored_commands.into_iter().map(|(_, cmd)| cmd).collect::<Vec<_>>()
    });

    use_effect(cx, filtered_commands, |cmds| {
        selected_index.set(0);
        async move { cmds }
    });

    if !app_state.read().command_palette_visible {
        return None;
    }

    let handle_keydown = move |evt: KeyboardEvent| {
        let current_filtered_commands = filtered_commands.read();
        if current_filtered_commands.is_empty() { return; }

        match evt.key() {
            Key::ArrowDown => selected_index.set((selected_index.get() + 1) % current_filtered_commands.len()),
            Key::ArrowUp => selected_index.set((selected_index.get() + current_filtered_commands.len() - 1) % current_filtered_commands.len()),
            Key::Enter => {
                if let Some(cmd_def) = current_filtered_commands.get(*selected_index.get()) {
                    // Clone necessary states for the closure
                    let app_state_clone = app_state.clone();
                    let app_config_clone = app_config.clone();
                    let engine_client_handle_clone = engine_client_handle.clone();
                    let filter_text_clone = filter_text.clone();

                    execute_command(cx, cmd_def.action.clone(), app_state_clone, app_config_clone, engine_client_handle_clone, filter_text_clone);
                }
            }
            Key::Escape => {
                app_state.write().command_palette_visible = false;
                filter_text.set(String::new());
            }
            _ => {}
        }
    };

    // execute_command needs cx to spawn tasks
    let execute_command = |
        cx: Scope,
        command: Command,
        app_state: UseSharedState<AppState>,
        app_config: UseSharedState<AppConfig>,
        engine_client_handle: UseSharedState<Option<EngineClient>>,
        filter_text_state: UseState<String>
    | {
        let mut app_state_writer = app_state.write();
        app_state_writer.command_palette_visible = false;
        filter_text_state.set(String::new());

        // Clone client for async task. If it's None, we can't proceed for network tasks.
        let client_guard = engine_client_handle.read();
        let maybe_client = client_guard.as_ref().cloned();

        match command {
            Command::LoadCsv { path } => {
                let file_to_load = path.unwrap_or_else(|| "tests/data/sample.csv".to_string()); // Default to sample
                let symbol = "WINFUT".to_string(); // For now, assume WINFUT for sample.csv

                if let Some(mut client) = maybe_client {
                    app_state_writer.is_loading = true;
                    app_state_writer.error_message = None;
                    drop(app_state_writer); // Release lock before await

                    let app_state_clone = app_state.clone();
                    cx.spawn(async move {
                        let mut app_state_writer_async = app_state_clone.write();
                        app_state_writer_async.clear_indicators_for_symbol(&symbol); // Clear old indicators

                        match client.load_csv(file_to_load.clone(), symbol.clone()).await {
                            Ok(load_msg) => {
                                tracing::info!("[COMMAND ACTION] Load CSV: {}", load_msg);
                                match client.get_market_data(symbol.clone()).await {
                                    Ok(candles_vec) => {
                                        let market_data = MarketData {
                                            symbol: symbol.clone(),
                                            candles: candles_vec,
                                            timeframe: shared::models::TimeFrame::Minute1, // Assuming, needs to be dynamic
                                        };
                                        app_state_writer_async.add_market_data(market_data);
                                        app_state_writer_async.set_display_data(&symbol);
                                        app_state_writer_async.error_message = None;
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Failed to get market data for {}: {}", symbol, e);
                                        tracing::error!("{}", err_msg);
                                        app_state_writer_async.error_message = Some(err_msg);
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Failed to load CSV {}: {}", file_to_load, e);
                                tracing::error!("{}", err_msg);
                                app_state_writer_async.error_message = Some(err_msg);
                            }
                        }
                        app_state_writer_async.is_loading = false;
                    });
                } else {
                    app_state_writer.error_message = Some("Engine client not available.".to_string());
                    tracing::warn!("[COMMAND ACTION] Engine client not available for Load CSV");
                }
            }
            Command::AddIndicator { indicator_type } => {
                let current_symbol = app_state_writer.current_symbol_display.clone();
                if let Some(mut client) = maybe_client {
                    if let Some(symbol) = current_symbol {
                        app_state_writer.is_loading = true;
                        app_state_writer.error_message = None;
                        drop(app_state_writer); // Release lock

                        let app_state_clone = app_state.clone();
                        let app_config_reader = app_config.read(); // Read once

                        // Determine parameters based on indicator_type from AppConfig
                        let params_json = match indicator_type.as_str() {
                            "SMA" => json!({"period": app_config_reader.indicators.sma.periods.get(0).unwrap_or(&20)}),
                            "EMA" => json!({"period": app_config_reader.indicators.ema.periods.get(0).unwrap_or(&9)}),
                            "RSI" => json!({"period": app_config_reader.indicators.rsi.period}),
                            _ => json!({}),
                        };
                        drop(app_config_reader); // Release lock

                        cx.spawn(async move {
                            let mut app_state_writer_async = app_state_clone.write();
                            match client.calculate_indicator(symbol.clone(), indicator_type.clone(), params_json.to_string()).await {
                                Ok(Some(indicator_data)) => {
                                    app_state_writer_async.add_indicator_to_symbol(&symbol, indicator_data);
                                    // set_display_data is called implicitly by add_indicator_to_symbol if symbol matches
                                    app_state_writer_async.error_message = None;
                                    tracing::info!("[COMMAND ACTION] Added indicator {} for {}", indicator_type, symbol);
                                }
                                Ok(None) => {
                                    let info_msg = format!("Indicator {} for {} returned no data.", indicator_type, symbol);
                                    tracing::info!("{}", info_msg);
                                    app_state_writer_async.error_message = Some(info_msg);
                                }
                                Err(e) => {
                                    let err_msg = format!("Failed to calculate indicator {} for {}: {}", indicator_type, symbol, e);
                                    tracing::error!("{}", err_msg);
                                    app_state_writer_async.error_message = Some(err_msg);
                                }
                            }
                            app_state_writer_async.is_loading = false;
                        });
                    } else {
                        app_state_writer.error_message = Some("No active symbol to add indicator to.".to_string());
                        tracing::warn!("[COMMAND ACTION] No active symbol for Add Indicator");
                    }
                } else {
                    app_state_writer.error_message = Some("Engine client not available.".to_string());
                    tracing::warn!("[COMMAND ACTION] Engine client not available for Add Indicator");
                }
            }
            Command::Exit => {
                tracing::info!("[COMMAND ACTION] Exit Application");
                dioxus_desktop::use_window(cx).close();
            }
            _ => { // Handle other commands like Configure, RemoveIndicator, Save/Load Project
                tracing::info!("[COMMAND ACTION] Command {:?} (Not fully implemented yet)", command);
            }
        }
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
