#![allow(new_without_default)]

use gio::{self, prelude::*, ApplicationFlags, SettingsBindFlags, SettingsExt, SimpleAction};
use glib;
use gtk;
use gtk::prelude::*;

use crossbeam_channel::{unbounded, Receiver, Sender};
use podcasts_data::Show;
use send_cell::SendCell;

use headerbar::Header;
use prefs::Prefs;
use settings::{self, WindowGeometry};
use stacks::{Content, PopulatedState};
use utils;
use widgets::about_dialog;
use widgets::appnotif::{InAppNotification, UndoState};
use widgets::player;
use widgets::show_menu::{mark_all_notif, remove_show_notif, ShowMenu};

use std::env;
use std::rc::Rc;
use std::sync::Arc;

pub const APP_ID: &str = "org.gnome.Podcasts";

/// Creates an action named $called in the action map $on with the handler $handle
macro_rules! action {
    ($on:expr, $called:expr, $handle:expr) => {{
        // Create a stateless, parameterless action
        let act = SimpleAction::new($called, None);
        // Connect the handler
        act.connect_activate($handle);
        // Add it to the map
        $on.add_action(&act);
        // Return the action
        act
    }};
}

#[derive(Debug, Clone)]
pub enum Action {
    RefreshAllViews,
    RefreshEpisodesView,
    RefreshEpisodesViewBGR,
    RefreshShowsView,
    ReplaceWidget(Arc<Show>),
    RefreshWidgetIfSame(i32),
    ShowWidgetAnimated,
    ShowShowsAnimated,
    HeaderBarShowTile(String),
    HeaderBarNormal,
    HeaderBarShowUpdateIndicator,
    HeaderBarHideUpdateIndicator,
    MarkAllPlayerNotification(Arc<Show>),
    RemoveShow(Arc<Show>),
    ErrorNotification(String),
    InitEpisode(i32),
    InitShowMenu(SendCell<ShowMenu>),
}

#[derive(Debug, Clone)]
pub struct App {
    instance: gtk::Application,
    window: gtk::ApplicationWindow,
    overlay: gtk::Overlay,
    settings: gio::Settings,
    content: Rc<Content>,
    headerbar: Rc<Header>,
    player: Rc<player::PlayerWidget>,
    sender: Sender<Action>,
    receiver: Receiver<Action>,
}

impl App {
    pub fn new(application: &gtk::Application) -> Rc<Self> {
        let settings = gio::Settings::new(APP_ID);

        let (sender, receiver) = unbounded();

        let window = gtk::ApplicationWindow::new(application);
        window.set_title("Podcasts");
        window.connect_delete_event(clone!(settings => move |window, _| {
            WindowGeometry::from_window(&window).write(&settings);
            Inhibit(false)
        }));

        // Create a content instance
        let content = Content::new(&sender).expect("Content Initialization failed.");

        // Create the headerbar
        let header = Header::new(&content, &sender);
        // Add the Headerbar to the window.
        window.set_titlebar(&header.container);

        // Add the content main stack to the overlay.
        let overlay = gtk::Overlay::new();
        overlay.add(&content.get_stack());

        let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
        // Add the overlay to the main Box
        wrap.add(&overlay);

        let player = player::PlayerWidget::new(&sender);
        // Add the player to the main Box
        wrap.add(&player.action_bar);

        window.add(&wrap);

        let app = App {
            instance: application.clone(),
            window,
            settings,
            overlay,
            headerbar: header,
            content,
            player,
            sender,
            receiver,
        };

        Rc::new(app)
    }

    fn init(app: &Rc<Self>) {
        let cleanup_date = settings::get_cleanup_date(&app.settings);
        // Garbage collect watched episodes from the disk
        utils::cleanup(cleanup_date);

        app.setup_gactions();
        app.setup_timed_callbacks();

        app.instance.connect_activate(move |_| ());

        // Retrieve the previous window position and size.
        WindowGeometry::from_settings(&app.settings).apply(&app.window);

        // Setup the Action channel
        gtk::timeout_add(25, clone!(app => move || app.setup_action_channel()));
    }

    fn setup_timed_callbacks(&self) {
        self.setup_dark_theme();
        self.setup_refresh_on_startup();
        self.setup_auto_refresh();
    }

    fn setup_dark_theme(&self) {
        let gtk_settings = gtk::Settings::get_default().unwrap();
        self.settings.bind(
            "dark-theme",
            &gtk_settings,
            "gtk-application-prefer-dark-theme",
            SettingsBindFlags::DEFAULT,
        );
    }

    fn setup_refresh_on_startup(&self) {
        // Update the feeds right after the Application is initialized.
        let sender = self.sender.clone();
        if self.settings.get_boolean("refresh-on-startup") {
            info!("Refresh on startup.");
            let s: Option<Vec<_>> = None;
            utils::refresh(s, sender.clone());
        }
    }

    fn setup_auto_refresh(&self) {
        let refresh_interval = settings::get_refresh_interval(&self.settings).num_seconds() as u32;
        info!("Auto-refresh every {:?} seconds.", refresh_interval);

        let sender = self.sender.clone();
        gtk::timeout_add_seconds(refresh_interval, move || {
            let s: Option<Vec<_>> = None;
            utils::refresh(s, sender.clone());

            glib::Continue(true)
        });
    }

    /// Define the `GAction`s.
    ///
    /// Used in menus and the keyboard shortcuts dialog.
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn setup_gactions(&self) {
        let sender = &self.sender;
        let win = &self.window;
        let instance = &self.instance;
        let header = &self.headerbar;
        let settings = &self.settings;

        // Create the `refresh` action.
        //
        // This will trigger a refresh of all the shows in the database.
        action!(win, "refresh", clone!(sender => move |_, _| {
            gtk::idle_add(clone!(sender => move || {
                let s: Option<Vec<_>> = None;
                utils::refresh(s, sender.clone());
                glib::Continue(false)
            }));
        }));
        self.instance.set_accels_for_action("win.refresh", &["<primary>r"]);

        // Create the `OPML` import action
        action!(win, "import", clone!(sender, win => move |_, _| {
            utils::on_import_clicked(&win, &sender)
        }));

        // Create the action that shows a `gtk::AboutDialog`
        action!(win, "about", clone!(win => move |_, _| about_dialog(&win)));

        // Create the quit action
        action!(win, "quit", clone!(instance => move |_, _| instance.quit()));
        self.instance.set_accels_for_action("win.quit", &["<primary>q"]);

        action!(
            win,
            "preferences",
            clone!(win, settings => move |_, _| {
                let dialog = Prefs::new(&settings);
                dialog.show(&win);
            })
        );
        self.instance.set_accels_for_action("win.preferences", &["<primary>e"]);

        // Create the menu action
        action!(win, "menu",clone!(header => move |_, _| header.open_menu()));
        // Bind the hamburger menu button to `F10`
        self.instance.set_accels_for_action("win.menu", &["F10"]);
    }

    fn setup_action_channel(&self) -> glib::Continue {
        if let Some(action) = self.receiver.try_recv() {
            trace!("Incoming channel action: {:?}", action);
            match action {
                Action::RefreshAllViews => self.content.update(),
                Action::RefreshShowsView => self.content.update_shows_view(),
                Action::RefreshWidgetIfSame(id) => self.content.update_widget_if_same(id),
                Action::RefreshEpisodesView => self.content.update_home(),
                Action::RefreshEpisodesViewBGR => self.content.update_home_if_background(),
                Action::ReplaceWidget(pd) => {
                    let shows = self.content.get_shows();
                    let mut pop = shows.borrow().populated();
                    pop.borrow_mut()
                        .replace_widget(pd.clone())
                        .map_err(|err| error!("Failed to update ShowWidget: {}", err))
                        .map_err(|_| error!("Failed ot update ShowWidget {}", pd.title()))
                        .ok();
                }
                Action::ShowWidgetAnimated => {
                    let shows = self.content.get_shows();
                    let mut pop = shows.borrow().populated();
                    pop.borrow_mut().switch_visible(
                        PopulatedState::Widget,
                        gtk::StackTransitionType::SlideLeft,
                    );
                }
                Action::ShowShowsAnimated => {
                    let shows = self.content.get_shows();
                    let mut pop = shows.borrow().populated();
                    pop.borrow_mut()
                        .switch_visible(PopulatedState::View, gtk::StackTransitionType::SlideRight);
                }
                Action::HeaderBarShowTile(title) => self.headerbar.switch_to_back(&title),
                Action::HeaderBarNormal => self.headerbar.switch_to_normal(),
                Action::HeaderBarShowUpdateIndicator => self.headerbar.show_update_notification(),
                Action::HeaderBarHideUpdateIndicator => self.headerbar.hide_update_notification(),
                Action::MarkAllPlayerNotification(pd) => {
                    let notif = mark_all_notif(pd, &self.sender);
                    notif.show(&self.overlay);
                }
                Action::RemoveShow(pd) => {
                    let notif = remove_show_notif(pd, self.sender.clone());
                    notif.show(&self.overlay);
                }
                Action::ErrorNotification(err) => {
                    error!("An error notification was triggered: {}", err);
                    let callback = || glib::Continue(false);
                    let notif = InAppNotification::new(&err, callback, || {}, UndoState::Hidden);
                    notif.show(&self.overlay);
                }
                Action::InitEpisode(rowid) => {
                    let res = self.player.initialize_episode(rowid);
                    debug_assert!(res.is_ok());
                }
                Action::InitShowMenu(s) => {
                    let menu = s.borrow();
                    self.headerbar.set_secondary_menu(&menu.container);
                }
            }
        }

        glib::Continue(true)
    }

    pub fn run() {
        let application = gtk::Application::new(APP_ID, ApplicationFlags::empty())
            .expect("Application Initialization failed...");

        application.connect_startup(clone!(application => move |_| {
            info!("CONNECT STARTUP RUN");
            let app = Self::new(&application);
            Self::init(&app);
            info!("Init complete");
            application.connect_activate(clone!(app => move |_| {
                info!("GApplication::activate");
                app.window.show_all();
                app.window.activate();
            }));
        }));

        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name("Podcasts");
        glib::set_prgname(Some("gnome-podcasts"));
        gtk::Window::set_default_icon_name(APP_ID);
        let args: Vec<String> = env::args().collect();
        ApplicationExtManual::run(&application, &args);
    }
}