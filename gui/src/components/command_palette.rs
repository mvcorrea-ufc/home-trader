// Command palette component (VSCode style)
#![allow(non_snake_case)]
use dioxus::prelude::*;

// As per spec:
// pub enum Command {
//     LoadCsv { path: String },
//     Configure,
//     Exit,
//     AddIndicator { indicator_type: String },
//     RemoveIndicator { name: String },
//     SaveProject { path: String },
//     LoadProject { path: String },
// }
//
// pub struct CommandPaletteState {
//     commands: Vec<CommandDefinition>, // CommandDefinition from spec
//     filter: String,
//     selected: usize,
//     is_visible: bool,
// }

#[component]
pub fn CommandPalette(cx: Scope) -> Element {
    // let is_visible = use_state(cx, || false);
    // let filter_text = use_state(cx, String::new);
    // let available_commands = use_state(cx, || /* load commands */);
    // let filtered_commands = use_memo(cx, (filter_text, available_commands), |(filter, cmds)| {
        // Use fuzzy_matcher to filter commands
    // });

    // if !is_visible.get() {
    //     return None;
    // }

    cx.render(rsx! {
        div {
            class: "command-palette-placeholder",
            // TODO: Implement input field, command list, fuzzy search
            input {
                // value: "{filter_text}",
                // oninput: move |evt| filter_text.set(evt.value.clone()),
                placeholder: "Type a command..."
            }
            ul {
                // for cmd in filtered_commands.read().iter() {
                //     rsx! { li { "{cmd.name}" } }
                // }
                li { "Example Command 1" }
                li { "Example Command 2" }
            }
            "Command Palette Placeholder"
        }
    })
}
