// Component for rendering technical indicators on the chart
#![allow(non_snake_case)]
use dioxus::prelude::*;
use shared::models::Indicator; // Import the Indicator struct

#[derive(Props, PartialEq, Clone)]
pub struct IndicatorOverlayProps {
    pub indicators: Vec<Indicator>,
    pub min_price: f64,
    pub max_price: f64,
    pub plot_height: f64,
    // pub plot_width: f64, // Not directly needed if candle_plot_width and num_candles are used for X
    pub margin_left: f64,
    pub margin_top: f64,
    pub candle_plot_width: f64, // Width allocated for each candle slot (body + spacing)
    pub num_candles_on_chart: usize, // To align indicator data points correctly
                                     // Optional: Global styling for indicators, or individual indicators can carry their style.
                                     // pub default_color: Option<String>,
                                     // pub default_stroke_width: Option<f64>,
}

#[component]
pub fn IndicatorOverlay(cx: Scope<IndicatorOverlayProps>) -> Element {
    if cx.props.indicators.is_empty() {
        return None; // Don't render anything if there are no indicators
    }

    let props = &cx.props;

    // Function to convert price to Y coordinate for indicators
    let price_range = if (props.max_price - props.min_price) > 0.0 { props.max_price - props.min_price } else { 1.0 };
    let y_scale_factor = props.plot_height / price_range;
    let price_to_y = |price: f64| props.margin_top + (props.max_price - price) * y_scale_factor;

    // Create SVG elements for each indicator
    let indicator_lines = props.indicators.iter().filter(|ind| !ind.values.is_empty()).map(|indicator| {
        let mut points = String::new();
        for (i, &value) in indicator.values.iter().enumerate() {
            // Ensure we don't try to plot more indicator points than candles visible.
            // Or, if indicator values can be sparse, this needs more sophisticated handling.
            // For now, assume indicator.values.len() <= num_candles_on_chart
            if i >= props.num_candles_on_chart { break; }

            // Calculate X: center of the candle slot
            let x = props.margin_left + (i as f64 * props.candle_plot_width) + (props.candle_plot_width / 2.0);
            let y = price_to_y(value);
            points.push_str(&format!("{:.2},{:.2} ", x, y));
        }
        points = points.trim_end().to_string(); // Remove trailing space

        // TODO: Use color from indicator data or AppConfig later
        let line_color = match indicator.name.to_lowercase().as_str() {
            "sma" => "#FFC107", // Amber
            "ema" => "#03A9F4", // Light Blue
            _ => "#9C27B0"      // Purple (default)
        };
        let stroke_width = 2.0; // TODO: Make configurable

        if points.is_empty() {
            None // Return None if no points were generated for this indicator
        } else {
            Some(rsx! {
                polyline {
                    points: "{points}",
                    fill: "none",
                    stroke: "{line_color}",
                    stroke_width: "{stroke_width}"
                }
            })
        }
    }).filter_map(|x| x); // Filter out None values if an indicator had no points

    cx.render(rsx! {
        g { // Group element for all indicator lines
            class: "indicator-overlay-group",
            indicator_lines
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
