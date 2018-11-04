// This file is part of perspektiv, a userspace daemon for graphically reporting
// system events.
// Copyright © 2018  Henrik Laxhuber <henrik@laxhuber.com>
//
// perspektiv is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License, version 3, as published by the
// Free Software Foundation.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

use gtk;
use gtk::{
    AdjustmentExt, Align::Center, ContainerExt, Continue, CssProviderExt, GtkWindowExt, Inhibit,
    LabelExt, Orientation, PositionType::*, ScaleExt, WidgetExt, WindowType::Popup,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use threlm::{Actor, Model, View};

use gdk::ScreenExt;

use glib::source::{source_remove, SourceId};
use glib::translate::{FromGlib, ToGlib, ToGlibPtr};

use config::{Config, MarginHoriz, MarginVert};
use subscribable::Subscribable;

pub use self::Msg::*;

pub struct Window {
    config: &'static Config,
    timeout: Option<SourceId>,
    widgets: Widgets,
}

#[allow(dead_code)]
struct Widgets {
    gtk_window: gtk::Window,
    outer_container: gtk::Box,
    container: gtk::Box,
    icon: gtk::Label,
    scale_adjustment: gtk::Adjustment,
    scale_widget: gtk::Scale,
    bool_label: gtk::Label,
}

/// Compute offset to $anchor from a dual-variant enum, where $opposite is the
/// variant that is not $anchor. $offset is spaced $distance from $anchor.
///
/// For example, if anchored to a variant `Left`, an input of $distance = 1080,
/// $value = Right(10) will yield 1070, which is the distance of Right(10) from
/// the anchor Left(0).
macro_rules! dimen {
    ($anchor:path, $opposite:path, $distance:expr, $value:expr) => {
        match $value {
            $anchor(v) => v,
            $opposite(v) => $distance - v,
        }
    };
}

impl Window {
    pub fn new(config: &'static Config) -> Self {
        let gtk_window = gtk::Window::new(Popup);
        gtk_window.set_name("window");

        // Module x11_backlight requires a GDK window, so we create an invisible
        // window and hide it immediately before continuing setup. The created
        // GDK window will remain and be used whenever gtk_window.show() is
        // called.
        gtk_window.set_property_default_width(0);
        gtk_window.set_property_default_height(0);
        gtk_window.show(); // create GDK window
        gtk_window.hide(); // and immediately hide

        // Need the screen for some configuration
        let screen = gtk_window
            .get_screen()
            .expect("Expected GTK window to have a GDK screen.");
        let monitor = screen.get_primary_monitor();
        let monitor_rect = screen.get_monitor_geometry(monitor);

        // Actually set up the window
        gtk_window.resize(config.window.width, config.window.height);
        gtk_window.set_resizable(false);
        gtk_window.move_(
            dimen!(
                MarginHoriz::Left,
                MarginHoriz::Right,
                monitor_rect.width - config.window.width,
                config.window.margin_horiz
            ),
            dimen!(
                MarginVert::Top,
                MarginVert::Bottom,
                monitor_rect.height - config.window.height,
                config.window.margin_vert
            ),
        );

        if config.window.opacity < 100 {
            gtk_window.set_opacity(config.window.opacity as f64 / 100.0);
        }

        // Topmost container holding child widgets in the window
        let outer_container = gtk::Box::new(Orientation::Vertical, config.window.spacing as i32);
        outer_container.set_name("outer_container");
        outer_container.set_valign(Center);
        outer_container.set_border_width(config.window.spacing);
        gtk_window.add(&outer_container);

        let icon = gtk::Label::new(None);
        icon.set_name("icon");
        outer_container.add(&icon);

        let container = gtk::Box::new(Orientation::Vertical, config.window.spacing as i32);
        container.set_name("container");
        outer_container.add(&container);

        let scale_adjustment = gtk::Adjustment::new(0.0, 0.0, 101.0, 1.0, 5.0, 1.0);
        let scale_widget = gtk::Scale::new(Orientation::Horizontal, &scale_adjustment);
        scale_widget.set_name("percentage");
        scale_widget.set_digits(0);
        scale_widget.set_draw_value(config.percentage.show_numeric);
        scale_widget.set_value_pos(Bottom);
        container.add(&scale_widget);

        let bool_label = gtk::Label::new(None);
        bool_label.set_name("boolean");
        container.add(&bool_label);

        outer_container.show_all();
        container.get_children().iter().for_each(|w| w.hide());

        let default_css = gtk::CssProvider::new();
        default_css
            .load_from_data(
                r#"
label#icon {
font-size: 36pt;
}
"#.as_bytes(),
            )
            .unwrap();
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &default_css,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        if let Some(ref path) = config.window.css {
            match path.as_path().to_str() {
                Some(path) => {
                    let css_provider = gtk::CssProvider::new();
                    if css_provider.load_from_path(path).is_err() {
                        error!(
                            "Failed to load CSS from custom path `{:#?}`",
                            path
                        );
                    } else {
                        gtk::StyleContext::add_provider_for_screen(
                            &screen,
                            &css_provider,
                            STYLE_PROVIDER_PRIORITY_APPLICATION,
                        );
                    }
                }
                None => error!(
                    "Custom CSS path `{:#?}` is not valid unicode",
                    path
                ),
            }
        }

        Window {
            config,
            timeout: None,
            widgets: Widgets {
                gtk_window,
                outer_container,
                container,
                icon,
                scale_adjustment,
                scale_widget,
                bool_label,
            },
        }
    }

    /// Hide the window after `config.window.duration` milliseconds.
    fn hide_timeout(&mut self, actor: Actor<Self>) {
        if let Some(ref id) = self.timeout {
            // Hacky, somewhat unsafe (owner of SourceId may not expect
            // it to invalidate, but in this specific situation it
            // should be fine as we set self.timeout to none in the
            // `Hide` branch) clone impl for glib::SourceId
            let id: u32 = id.to_glib();
            let id: SourceId = SourceId::from_glib(id);
            source_remove(id);
        }
        self.timeout = Some(gtk::timeout_add(self.config.window.duration, move || {
            actor.tell(Hide).unwrap();
            Continue(false)
        }));
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    ShowPercent(&'static str, f64),
    ShowBool(&'static str, &'static str),
    Hide,
    Quit,
}

impl Model for Window {
    type Message = Msg;

    fn connect(&self, actor: Actor<Self>) {
        self.widgets.gtk_window.connect_delete_event({
            let actor = actor.clone();
            move |_, _| {
                actor.tell(Quit).unwrap();
                Inhibit(false)
            }
        });

        // Subscribe to modules
        macro_rules! subscribe {
            ($module:ident, $params:expr) => {
                $crate::$module::Subscription::subscribe(
                    actor.clone(),
                    stringify!($module),
                    $params,
                );
            };
        }

        #[cfg(feature = "x11_backlight")]
        {
            let window = self
                .widgets
                .gtk_window
                .get_window()
                .expect("Expected GTK Window to have a GDK window")
                .to_glib_none()
                .0;
            let window = unsafe { ::x11_backlight::gdk_x11_window_get_xid(window) };
            subscribe!(x11_backlight, window);
        }

        #[cfg(feature = "alsa_volume")]
        subscribe!(alsa_volume, ());
    }

    fn update(&mut self, msg: Self::Message, actor: Actor<Self>) {
        match msg {
            ShowPercent(icon, value) => {
                self.widgets
                    .container
                    .get_children()
                    .iter()
                    .for_each(|w| w.hide());

                self.widgets.icon.set_text(icon);
                self.widgets.scale_adjustment.set_value(value * 100.0);

                self.widgets.scale_widget.show();
                self.widgets.gtk_window.show();

                self.hide_timeout(actor);
            }
            ShowBool(icon, label) => {
                self.widgets
                    .container
                    .get_children()
                    .iter()
                    .for_each(|w| w.hide());

                self.widgets.icon.set_text(icon);
                if self.config.boolean.show_label {
                    self.widgets.bool_label.set_text(label);
                    self.widgets.bool_label.show();
                }
                self.widgets.gtk_window.show();

                self.hide_timeout(actor);
            }
            Hide => {
                self.timeout = None;
                self.widgets.gtk_window.hide();
            }
            Quit => {
                gtk::main_quit();
            }
        }
    }
}

impl View for Window {
    type Root = gtk::Window;

    fn root(&self) -> &Self::Root {
        &self.widgets.gtk_window
    }
}
