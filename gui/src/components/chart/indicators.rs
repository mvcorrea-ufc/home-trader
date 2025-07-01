// Component for rendering technical indicators on the chart
#![allow(non_snake_case)]
use dioxus::prelude::*;
use shared::models::Indicator; // Import the Indicator struct

// Removed manual Props struct definition
// #[derive(Props, PartialEq, Clone)]
// pub struct IndicatorOverlayProps {
//     pub indicators: Vec<Indicator>,
//     pub min_price: f64,
//     pub max_price: f64,
//     pub plot_height: f64,
//     pub margin_left: f64,
//     pub margin_top: f64,
//     pub candle_plot_width: f64,
//     pub num_candles_on_chart: usize,
// }

#[component]
pub fn IndicatorOverlay(
    // Props are now direct function arguments
    indicators: Vec<Indicator>,
    min_price: f64,
    max_price: f64,
    plot_height: f64,
    margin_left: f64,
    margin_top: f64,
    candle_plot_width: f64,
    num_candles_on_chart: usize,
) -> Element {
    if indicators.is_empty() {
        return None;
    }

    // Access props directly by their names
    let price_range = if (max_price - min_price) > 0.0 { max_price - min_price } else { 1.0 };
    let y_scale_factor = plot_height / price_range;
    let price_to_y = |price_val: f64| margin_top + (max_price - price_val) * y_scale_factor;

    let indicator_line_elements = indicators.iter().filter(|ind| !ind.values.is_empty()).map(|indicator| {
        let mut points = String::new();
        for (i, &value) in indicator.values.iter().enumerate() {
            if i >= num_candles_on_chart { break; }

            let x = margin_left + (i as f64 * candle_plot_width) + (candle_plot_width / 2.0);
            let y = price_to_y(value);
            points.push_str(&format!("{:.2},{:.2} ", x, y));
        }
        points = points.trim_end().to_string();

        let line_color = match indicator.name.to_lowercase().as_str() {
            "sma" => "#FFC107",
            "ema" => "#03A9F4",
            _ => "#9C27B0"
        };
        let stroke_width_val = 2.0;

        if points.is_empty() {
            None
        } else {
            Some(rsx! {
                polyline {
                    points: "{points}",
                    fill: "none",
                    stroke: "{line_color}",
                    stroke_width: "{stroke_width_val}"
                }
            })
        }
    }).filter_map(|x| x);

    // Return rsx! directly
    rsx! {
        g {
            class: "indicator-overlay-group",
            {indicator_line_elements} // Render the iterator of Elements
        }
        // Placeholder text removed, actual lines will be rendered.
        // If needed for debugging specific props:
        /*
        text {
            x: "{props.margin_left + 10.0}",
            y: "{props.margin_top + 60.0}",
            fill: "#88f",
            font_size: "12px",
            "Indicator(0) Name: {props.indicators.first().map_or("N/A", |i| i.name.as_str())}, Values: {props.indicators.first().map_or(0, |i| i.values.len())}"
        }
        */
    })
}
