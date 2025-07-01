// Candlestick chart rendering component
#![allow(non_snake_case)]
use dioxus::prelude::*;
use shared::models::Candle; // Import the Candle struct

// This will be a complex component. For now, a simple placeholder.
// It will need to:
// - Take market data (Vec<Candle>) as input.
// - Render SVG or Canvas elements for candles, wicks, volume bars.
// - Handle zooming, panning, and scaling.
// - Potentially interact with other components like indicator overlays.

#[derive(Props, PartialEq, Clone)]
pub struct CandlestickChartProps {
    pub candles: Vec<Candle>,
    #[props(default = 800.0)] // Default width
    pub width: f64,
    #[props(default = 400.0)] // Default height
    pub height: f64,
    // TODO: Add other config like colors from AppConfig later
}

#[component]
pub fn CandlestickChart(cx: Scope<CandlestickChartProps>) -> Element {
    if cx.props.candles.is_empty() {
        return cx.render(rsx! {
            div {
                style: "width: {cx.props.width}px; height: {cx.props.height}px; display: flex; align-items: center; justify-content: center; border: 1px solid #ccc; background-color: #f0f0f0;",
                "No candle data available."
            }
        });
    }

    let candles = &cx.props.candles;
    let chart_width = cx.props.width;
    let chart_height = cx.props.height;

    // Define margins
    let margin_top = 20.0;
    let margin_bottom = 30.0;
    let margin_left = 50.0;
    let margin_right = 20.0;

    let plot_width = chart_width - margin_left - margin_right;
    let plot_height = chart_height - margin_top - margin_bottom;

    // Determine price range
    let mut min_price = candles.first().map_or(0.0, |c| c.low);
    let mut max_price = candles.first().map_or(0.0, |c| c.high);
    for candle in candles.iter() {
        if candle.low < min_price {
            min_price = candle.low;
        }
        if candle.high > max_price {
            max_price = candle.high;
        }
    }
    // Add some padding to min/max price for better visualization
    let price_padding = (max_price - min_price) * 0.05; // 5% padding
    min_price -= price_padding;
    max_price += price_padding;
    if min_price < 0.0 { min_price = 0.0; } // Ensure min_price is not negative if data is close to zero

    let price_range = if (max_price - min_price) > 0.0 { max_price - min_price } else { 1.0 }; // Avoid division by zero

    // Scaling factors
    let y_scale_factor = plot_height / price_range;
    // Function to convert price to Y coordinate
    // Y is inverted in SVG (0 is top), so (max_price - price_value)
    let price_to_y = |price: f64| margin_top + (max_price - price) * y_scale_factor;

    let num_candles = candles.len() as f64;
    let candle_plot_width = plot_width / num_candles; // Includes spacing
    let candle_width = (candle_plot_width * 0.7).max(1.0); // Candle body is 70% of its allocated space, min 1px
    let candle_spacing = candle_plot_width - candle_width;


    let candle_elements = candles.iter().enumerate().map(|(i, candle)| {
        let x_base = margin_left + (i as f64 * candle_plot_width);
        let candle_x = x_base + candle_spacing / 2.0;

        let body_top_price = candle.open.max(candle.close);
        let body_bottom_price = candle.open.min(candle.close);

        let body_y = price_to_y(body_top_price);
        let body_height = (body_top_price - body_bottom_price) * y_scale_factor;
        // Ensure body_height is at least 1px if open and close are very close, but not identical
        let body_height = if body_height < 1.0 && candle.open != candle.close { 1.0 } else { body_height.max(0.0) };


        let wick_top_y = price_to_y(candle.high);
        let wick_bottom_y = price_to_y(candle.low);
        let wick_x_center = candle_x + candle_width / 2.0;

        // Standard trading colors
        let bullish_color = "#26a69a"; // Greenish
        let bearish_color = "#ef5350"; // Reddish
        let color = if candle.close >= candle.open { bullish_color } else { bearish_color };

        rsx! {
            // Wick
            line {
                x1: "{wick_x_center}",
                y1: "{wick_top_y}",
                x2: "{wick_x_center}",
                y2: "{wick_bottom_y}",
                stroke: color,
                stroke_width: "1"
            }
            // Candle Body
            rect {
                x: "{candle_x}",
                y: "{body_y}",
                width: "{candle_width}",
                height: "{body_height}",
                fill: color,
                // Optional: add a stroke to the body
                // stroke: "black",
                // stroke_width: "0.5"
            }
        }
    });

    cx.render(rsx! {
        div {
            class: "candlestick-chart-container",
            style: "width: {chart_width}px; height: {chart_height}px; border: 1px solid #444; background-color: #222; color: #eee;",
            svg {
                width: "{chart_width}",
                height: "{chart_height}",
                // Background for the plot area
                rect {
                    x: "{margin_left}",
                    y: "{margin_top}",
                    width: "{plot_width}",
                    height: "{plot_height}",
                    fill: "#2a2a2a"
                }
                // Group for actual candle elements
                g {
                    // The transform for the group itself is not strictly needed here
                    // as coordinates are calculated relative to the SVG root.
                    // However, individual elements are positioned using margin_left and margin_top logic.
                    candle_elements
                }
                // Remove placeholder text or comment out
                /*
                text {
                    x: "{margin_left + 10.0}",
                    y: "{margin_top + 20.0}",
                    fill: "#aaa",
                    font_size: "12px",
                    "Min Price: {min_price:.2}, Max Price: {max_price:.2}, Y-Scale: {y_scale_factor:.2}"
                }
                text {
                    x: "{margin_left + 10.0}",
                    y: "{margin_top + 40.0}",
                    fill: "#aaa",
                    font_size: "12px",
                    "Candle Width: {candle_width:.2}, Spacing: {candle_spacing:.2}"
                }
                */
                // TODO: Add Axes (numbers for price and time)
            }
        }
    })
}
