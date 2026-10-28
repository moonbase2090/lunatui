// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(feature = "barchart")]
mod bar_chart;
#[cfg(feature = "bigtext")]
mod big_text;
mod block;
mod borders;
#[cfg(feature = "calendar")]
mod calendar;
#[cfg(feature = "chart")]
mod chart;
mod gauge;
#[cfg(feature = "image")]
mod image;
mod lightcycle;
mod list;
mod overlay;
mod padding;
mod paragraph;
mod render_cx;
mod scrollbar;
#[cfg(feature = "sparkline")]
mod sparkline;
mod spinner;
mod spinner_preset;
#[cfg(feature = "widget-state")]
mod state;
mod status_bar;
mod status_dot;
mod table;
mod tabs;
mod text_input;
mod widget;

#[cfg(feature = "barchart")]
pub use bar_chart::{Bar, BarChart, BarDirection, BarGroup};
#[cfg(feature = "bigtext")]
pub use big_text::{BigText, BigTextFont, PixelSize};
pub use block::{Alignment, Block, TitlePosition};
pub use borders::{BorderType, Borders};
#[cfg(feature = "calendar")]
pub use calendar::{Date, DateStyle, Monthly, WeekStart};
#[cfg(feature = "chart")]
pub use chart::{Axis, Chart, Dataset, GraphType, LegendPosition, Marker};
pub use gauge::Gauge;
#[cfg(feature = "image")]
pub use image::Image;
pub use lightcycle::Lightcycle;
pub use list::List;
pub use lunatui_core::Size;
#[cfg(feature = "image")]
pub use lunatui_image::{Fit as ImageFit, ImageError, ImageId, ImageSource};
pub use lunatui_layout::Constraint;
pub use lunatui_text::{RowBuilder, RowSource, VecSource};
pub use overlay::{FocusTrap, Overlay, blend_rgb};
pub use padding::Padding;
pub use paragraph::{Paragraph, TextView, TextWrap};
pub use render_cx::RenderCx;
pub use scrollbar::Scrollbar;
#[cfg(feature = "sparkline")]
pub use sparkline::{
    BarSet, Baseline, NINE_LEVELS, RenderDirection, Sparkline, SparklineBar, SparklineValue,
    THREE_LEVELS,
};
pub use spinner::{Spinner, SpinnerPattern};
pub use spinner_preset::SpinnerPreset;
#[cfg(feature = "widget-state")]
pub use state::WidgetState;
pub use status_bar::StatusBar;
pub use status_dot::{StatusDot, StatusState};
pub use table::Table;
pub use tabs::Tabs;
pub use text_input::TextInput;
pub use widget::Widget;
