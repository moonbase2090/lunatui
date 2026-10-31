// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(feature = "accordion")]
mod accordion;
#[cfg(any(feature = "tooltip", feature = "popover"))]
mod anchor;
#[cfg(feature = "barchart")]
mod bar_chart;
#[cfg(feature = "bigtext")]
mod big_text;
mod block;
mod borders;
mod breadcrumbs;
#[cfg(feature = "button")]
mod button;
#[cfg(feature = "calendar")]
mod calendar;
/// Shapes for [`Canvas`] and the painter they draw with.
#[cfg(feature = "canvas")]
pub mod canvas;
#[cfg(feature = "chart")]
mod chart;
#[cfg(feature = "combo-box")]
mod combo_box;
#[cfg(feature = "date-picker")]
mod date_picker;
#[cfg(feature = "dialog")]
mod dialog;
#[cfg(feature = "diff-view")]
mod diff_view;
#[cfg(feature = "form")]
mod form;
mod gauge;
#[cfg(feature = "graphite-render")]
pub mod graphite;
#[cfg(feature = "hex-viewer")]
mod hex_viewer;
#[cfg(feature = "image")]
mod image;
#[cfg(feature = "key-hints")]
mod key_hint_bar;
mod lightcycle;
#[cfg(feature = "line-gauge")]
mod line_gauge;
mod list;
#[cfg(feature = "log-view")]
mod log_view;
#[cfg(feature = "markdown")]
mod markdown_view;
#[cfg(feature = "nested-menu")]
mod nested_menu;
#[cfg(feature = "notification-center")]
mod notification_center;
#[cfg(feature = "number-input")]
mod number_input;
mod overlay;
mod padding;
mod paragraph;
#[cfg(feature = "pie-chart")]
mod pie_chart;
#[cfg(feature = "popover")]
mod popover;
#[cfg(feature = "popup")]
mod popup;
#[cfg(feature = "progress")]
mod progress_bar;
#[cfg(feature = "radio")]
mod radio_group;
#[cfg(any(feature = "chart", feature = "canvas", feature = "pie-chart"))]
mod raster;
mod render_cx;
#[cfg(feature = "scroll-view")]
mod scroll_view;
mod scrollbar;
#[cfg(feature = "search-box")]
mod search_box;
#[cfg(feature = "slider")]
mod slider;
#[cfg(feature = "sparkline")]
mod sparkline;
mod spinner;
mod spinner_preset;
mod split;
#[cfg(feature = "widget-state")]
mod state;
mod status_bar;
mod status_dot;
mod table;
mod table_state;
mod tabs;
mod text_input;
#[cfg(feature = "toast")]
mod toast;
#[cfg(feature = "toggle")]
mod toggle;
#[cfg(feature = "tooltip")]
mod tooltip;
mod widget;

#[cfg(feature = "accordion")]
pub use accordion::{Accordion, AccordionSection, SectionArea};
#[cfg(any(feature = "tooltip", feature = "popover"))]
pub use anchor::AnchorSide;
#[cfg(feature = "barchart")]
pub use bar_chart::{Bar, BarChart, BarDirection, BarGroup};
#[cfg(feature = "bigtext")]
pub use big_text::{BigText, BigTextFont, PixelSize};
pub use block::{Alignment, Block, TitlePosition};
pub use borders::{BorderType, Borders};
pub use breadcrumbs::Breadcrumbs;
#[cfg(feature = "button")]
pub use button::{Button, ButtonState, ButtonVariant};
#[cfg(feature = "calendar")]
pub use calendar::{Date, DateStyle, Monthly, WeekStart};
#[cfg(feature = "canvas")]
pub use canvas::Canvas;
#[cfg(feature = "chart")]
pub use chart::{Axis, Chart, Dataset, GraphType, LegendPosition};
#[cfg(feature = "combo-box")]
pub use combo_box::ComboBox;
#[cfg(feature = "date-picker")]
pub use date_picker::DatePicker;
#[cfg(feature = "dialog")]
pub use dialog::{Dialog, DialogClosed, DialogFocus, DialogKind, DialogOutcome, DialogState};
#[cfg(feature = "diff-view")]
pub use diff_view::{Diff, DiffHunk, DiffKind, DiffLayout, DiffLine, DiffView};
#[cfg(feature = "form")]
pub use form::{FieldArea, Form, FormField};
pub use gauge::Gauge;
#[cfg(feature = "hex-viewer")]
pub use hex_viewer::{HexViewer, HexViewerState};
#[cfg(feature = "image")]
pub use image::Image;
#[cfg(feature = "key-hints")]
pub use key_hint_bar::KeyHintBar;
pub use lightcycle::Lightcycle;
#[cfg(feature = "line-gauge")]
pub use line_gauge::LineGauge;
pub use list::List;
#[cfg(feature = "log-view")]
pub use log_view::{LogBuffer, LogEntry, LogLevel, LogView};
pub use lunatui_core::Size;
#[cfg(feature = "image")]
pub use lunatui_image::{Fit as ImageFit, ImageError, ImageId, ImageSource};
pub use lunatui_layout::Constraint;
pub use lunatui_text::{RowBuilder, RowSource, VecSource};
#[cfg(feature = "markdown")]
pub use markdown_view::MarkdownView;
#[cfg(feature = "nested-menu")]
pub use nested_menu::{Menu, MenuEvent, MenuItem, MenuState};
#[cfg(feature = "notification-center")]
pub use notification_center::{Notification, NotificationCenter, NotificationId, NotificationLog};
#[cfg(feature = "number-input")]
pub use number_input::NumberInput;
pub use overlay::{FocusTrap, Overlay, blend_rgb};
pub use padding::Padding;
pub use paragraph::{Paragraph, TextView, TextWrap};
#[cfg(feature = "pixel-pie-chart")]
pub use pie_chart::PixelPieChart;
#[cfg(feature = "pie-chart")]
pub use pie_chart::{PieChart, PieLayout, PieSlice};
#[cfg(feature = "popover")]
pub use popover::{Popover, PopoverClick, PopoverState};
#[cfg(feature = "popup")]
pub use popup::{Placement, Popup, PopupState};
#[cfg(feature = "progress")]
pub use progress_bar::{ProgressBar, ProgressTone};
#[cfg(feature = "radio")]
pub use radio_group::{RadioFlow, RadioGroup};
#[cfg(any(feature = "chart", feature = "canvas", feature = "pie-chart"))]
pub use raster::Marker;
pub use render_cx::RenderCx;
#[cfg(feature = "scroll-view")]
pub use scroll_view::{ScrollView, ScrollViewState, ScrollbarVisibility};
pub use scrollbar::Scrollbar;
#[cfg(feature = "search-box")]
pub use search_box::{SearchBox, SearchMatch};
#[cfg(feature = "slider")]
pub use slider::Slider;
#[cfg(feature = "sparkline")]
pub use sparkline::{
    BarSet, Baseline, NINE_LEVELS, RenderDirection, Sparkline, SparklineBar, SparklineValue,
    THREE_LEVELS,
};
pub use spinner::{Spinner, SpinnerPattern};
pub use spinner_preset::SpinnerPreset;
pub use split::{
    Divider, DividerId, PaneArea, PaneId, SplitAxis, SplitDrag, SplitLayout, SplitTree,
};
#[cfg(feature = "widget-state")]
pub use state::WidgetState;
pub use status_bar::StatusBar;
pub use status_dot::{StatusDot, StatusState};
pub use table::{CellStyle, Table};
pub use table_state::{SortDirection, TableState, TableView};
pub use tabs::{TabMove, Tabs, TabsDrag};
pub use text_input::TextInput;
#[cfg(feature = "toast")]
pub use toast::{Toast, ToastId, ToastKind, ToastPlacement, ToastScope, ToastStack};
#[cfg(feature = "toggle")]
pub use toggle::Toggle;
#[cfg(feature = "tooltip")]
pub use tooltip::{Tooltip, TooltipState};
pub use widget::Widget;
