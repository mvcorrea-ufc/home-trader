// Command palette component (VSCode style)
#![allow(non_snake_case)]
use dioxus::prelude::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::state::app_state::AppState;
use crate::config::AppConfig; // Import AppConfig
use crate::services::engine_client::EngineClient; // Import EngineClient
use shared::models::MarketData; // MarketData is used. Candle & Indicator are part of it but not directly typed here.
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
pub fn CommandPalette() -> Element { // Removed cx: Scope
    // Hooks no longer take cx as the first argument in Dioxus 0.5+
    let app_state = use_shared_state::<AppState>().unwrap();
    let app_config = use_shared_state::<AppConfig>().unwrap();
    let engine_client_handle = use_shared_state::<Option<EngineClient>>().unwrap();
    let window_handle = dioxus_desktop::use_window(); // Call use_window at the top level and store the handle

    let all_commands = use_ref(|| { // Removed cx
        vec![
            CommandDefinition::new(0, "Load CSV Data (Sample WINFUT)", "Import WINFUT market data from a sample CSV file", Command::LoadCsv { path: Some("tests/data/sample.csv".to_string()) }),
            CommandDefinition::new(1, "Add Indicator: SMA", "Add Simple Moving Average indicator", Command::AddIndicator { indicator_type: "SMA".to_string() }),
            CommandDefinition::new(2, "Add Indicator: EMA", "Add Exponential Moving Average indicator", Command::AddIndicator { indicator_type: "EMA".to_string() }),
            CommandDefinition::new(3, "Add Indicator: RSI", "Add Relative Strength Index indicator", Command::AddIndicator { indicator_type: "RSI".to_string() }),
            CommandDefinition::new(4, "Exit Application", "Close Home Trader", Command::Exit),
            // More commands...
        ]
    });

    let filter_text = use_state(String::new); // Removed cx
    let selected_index = use_state(|| 0usize); // Removed cx
    let matcher = use_ref(SkimMatcherV2::default); // Removed cx

    // Corrected use_memo: dependencies are in a tuple, closure takes the destructured tuple.
    // To react to filter_text (UseState) and all_commands (UseRef), we clone/read their current values for the dependency array.
    let current_filter_text_for_memo = filter_text.current().clone();
    // Depending on all_commands.read() directly in dependency array is tricky as it's a Ref a Vec, not easily comparable for changes.
    // A common way is to use a "version" or length if the content of all_commands can change, or assume it's static.
    // For now, assume all_commands is static after init for simplicity of memo.
    let filtered_commands = use_memo((current_filter_text_for_memo,), move |(current_filter_text,)| {
        let cmds = all_commands.read();
        if current_filter_text.is_empty() {
            return cmds.clone();
        }
        let mut scored_commands: Vec<(i64, CommandDefinition)> = cmds
            .iter()
            .filter_map(|cmd| {
                matcher.read().fuzzy_match(&cmd.name, &current_filter_text)
                    .map(|score| (score, cmd.clone()))
            })
            .collect();

        scored_commands.sort_by(|a, b| b.0.cmp(&a.0));
        scored_commands.into_iter().map(|(_, cmd)| cmd).collect::<Vec<_>>()
    });

    // Corrected use_effect: dependencies in tuple, closure takes destructured tuple.
    // The async move {cmds} was incorrect for a simple effect.
    // To react to filtered_commands (UseMemo), we use its current value in dependency.
    let current_filtered_commands_for_effect = filtered_commands.current();
    let current_filtered_commands_len = current_filtered_commands_for_effect.len();

    use_effect((current_filtered_commands_len,), move |(_len,)| { // Removed cx
        selected_index.set(0);
        // No return or async block needed if just setting state.
        // If cleanup is needed, return a closure: || { /* cleanup */ }
    });

    if !app_state.read().command_palette_visible {
        return None;
    }

    // Define execute_command as a closure that captures necessary context, wrapped in Rc
    let execute_command_closure = std::rc::Rc::new({
        // Clone handles that need to be captured by the closure itself.
        // These will be further cloned if needed for spawned async tasks within the closure.
        let app_state_captured = app_state.clone();
        let app_config_captured = app_config.clone();
        let engine_client_handle_captured = engine_client_handle.clone();
        let filter_text_captured = filter_text.clone();
        let window_handle_captured = window_handle.clone();

        move |command: Command| {
            let mut app_state_writer = app_state_captured.write();
            app_state_writer.command_palette_visible = false;
            filter_text_captured.set(String::new());

            let client_guard = engine_client_handle_captured.read();
            let maybe_client = client_guard.as_ref().cloned();

            match command {
                Command::LoadCsv { path } => {
                    let file_to_load = path.unwrap_or_else(|| "tests/data/sample.csv".to_string());
                    let symbol = "WINFUT".to_string();

                    if let Some(mut client) = maybe_client {
                        app_state_writer.is_loading = true;
                        app_state_writer.error_message = None;
                        drop(app_state_writer); // Release lock before await

                        let app_state_async = app_state_captured.clone();
                        spawn(async move { // Use dioxus::prelude::spawn
                            let mut app_state_writer_async = app_state_async.write();
                            app_state_writer_async.clear_indicators_for_symbol(&symbol);

                            match client.load_csv(file_to_load.clone(), symbol.clone()).await {
                                Ok(load_msg) => {
                                    tracing::info!("[COMMAND ACTION] Load CSV: {}", load_msg);
                                    drop(app_state_writer_async); // Release before next await
                                    let data_result = client.get_market_data(symbol.clone()).await;
                                    app_state_writer_async = app_state_async.write(); // Re-acquire

                                    match data_result {
                                        Ok(candles_vec) => {
                                            let market_data = MarketData {
                                                symbol: symbol.clone(),
                                                candles: candles_vec,
                                                timeframe: shared::models::TimeFrame::Minute1,
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

                            let app_config_reader = app_config_captured.read();
                            let params_json = match indicator_type.as_str() {
                                "SMA" => json!({"period": app_config_reader.indicators.sma.periods.get(0).unwrap_or(&20)}),
                                "EMA" => json!({"period": app_config_reader.indicators.ema.periods.get(0).unwrap_or(&9)}),
                                "RSI" => json!({"period": app_config_reader.indicators.rsi.period}),
                                _ => json!({}),
                            };
                            drop(app_config_reader);
                            drop(app_state_writer); // Release lock

                            let app_state_async = app_state_captured.clone();
                            spawn(async move { // Use dioxus::prelude::spawn
                                let mut app_state_writer_async = app_state_async.write();
                                match client.calculate_indicator(symbol.clone(), indicator_type.clone(), params_json.to_string()).await {
                                    Ok(Some(indicator_data)) => {
                                        app_state_writer_async.add_indicator_to_symbol(&symbol, indicator_data);
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
                    window_handle_captured.close(); // Use the captured window_handle
                }
                _ => {
                    tracing::info!("[COMMAND ACTION] Command {:?} (Not fully implemented yet)", command);
                }
            }
        }
    };

    let handle_keydown = move |evt: KeyboardEvent| {
        let current_filtered_cmds = filtered_commands.current(); // Get current value of memoized result
        if current_filtered_cmds.is_empty() { return; }

        match evt.key() {
            Key::ArrowDown => selected_index.set((selected_index.get() + 1) % current_filtered_cmds.len()),
            Key::ArrowUp => selected_index.set((selected_index.get() + current_filtered_cmds.len() - 1) % current_filtered_cmds.len()),
            Key::Enter => {
                if let Some(cmd_def) = current_filtered_cmds.get(*selected_index.get()) {
                    execute_command_closure(cmd_def.action.clone()); // Call the new closure
                }
            }
            Key::Escape => {
                app_state.write().command_palette_visible = false; // app_state is captured by handle_keydown
                filter_text.set(String::new()); // filter_text is captured by handle_keydown
            }
            _ => {}
        }
    };

    // execute_command_closure is now Rc<impl Fn(Command)>, so it can be cloned for each li.

    rsx! {
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
                                key: "{current_cmd_def.id}",
                                style: "padding: 10px 12px; border-bottom: 1px solid #444; cursor: pointer; background-color: {bg_color}; border-radius: 3px;",
                                onclick: {
                                    let ecc_for_onclick = execute_command_closure.clone(); // Clone Rc handle
                                    let action_for_onclick = current_cmd_def.action.clone();
                                    move |_| ecc_for_onclick(action_for_onclick.clone()) // action might need to be cloned again if called multiple times
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
