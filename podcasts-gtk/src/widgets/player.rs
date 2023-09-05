// player.rs
//
// Copyright 2018 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gst::ClockTime;

use gtk::prelude::*;
use gtk::{gio, glib};

use gio::File;
use glib::clone;
use glib::{SignalHandlerId, WeakRef};

use anyhow::Result;
use chrono::{prelude::*, NaiveTime};
use fragile::Fragile;
use glib::Sender;
use once_cell::sync::Lazy;
use url::Url;

use podcasts_data::{dbqueries, downloader, EpisodeWidgetModel, ShowCoverModel, USER_AGENT};

use crate::app::Action;
use crate::config::APP_ID;
use crate::utils::set_image_from_path;

use std::cell::{RefCell, RefMut};
use std::convert::TryInto;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

use crate::i18n::i18n;

use mpris_player::{Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
enum SeekDirection {
    Backwards,
    Forward,
}

trait PlayerExt {
    fn play(&self);
    fn pause(&mut self);
    fn stop(&mut self);
    fn seek(&self, offset: ClockTime, direction: SeekDirection) -> Option<()>;
    fn fast_forward(&self);
    fn rewind(&self);
    fn set_playback_rate(&self, _: f64);
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    show: gtk::Label,
    episode: gtk::Label,
    cover: gtk::Image,
    show_small: gtk::Label,
    episode_small: gtk::Label,
    cover_small: gtk::Image,
    mpris: Arc<MprisPlayer>,
    restore_position: i32,
    finished_restore: bool,
    ep: Option<EpisodeWidgetModel>,
    episode_id: RefCell<Option<i32>>,
}

impl PlayerInfo {
    fn create_bindings(&self) {
        self.show
            .bind_property("label", &self.show_small, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        self.episode
            .bind_property("label", &self.episode_small, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    // FIXME: create a Diesel Model of the joined episode and podcast query instead
    fn init(&mut self, episode: &EpisodeWidgetModel, podcast: &ShowCoverModel) {
        self.ep = Some(episode.clone());
        self.episode_id.replace(Some(episode.rowid()));
        self.set_cover_image(podcast);
        self.set_show_title(podcast);
        self.set_episode_title(episode);

        let mut metadata = Metadata::new();
        metadata.artist = Some(vec![podcast.title().to_string()]);
        metadata.title = Some(episode.title().to_string());

        // Set the cover if it is already cached.
        if let Some(path) = downloader::check_for_cached_cover(podcast)
            .as_ref()
            .and_then(|p| p.to_str())
        {
            let url = Url::from_file_path(path);
            metadata.art_url = url.map(From::from).ok();
        } else {
            // fallback: set the cover to the http url if it isn't cached, yet.
            // TODO we could trigger an async download of the cover here
            // and update the metadata when it's done.
            metadata.art_url = podcast.image_uri().map(From::from);
        }

        self.mpris.set_metadata(metadata);
        self.mpris.set_can_play(true);
        self.mpris.set_can_seek(true);
    }

    fn set_episode_title(&self, episode: &EpisodeWidgetModel) {
        self.episode.set_text(episode.title());
        self.episode.set_tooltip_text(Some(episode.title()));
    }

    fn set_show_title(&self, show: &ShowCoverModel) {
        self.show.set_text(show.title());
        self.show.set_tooltip_text(Some(show.title()));
    }

    fn set_cover_image(&self, show: &ShowCoverModel) {
        set_image_from_path(&self.cover, show.id(), 34)
            .map_err(|err| error!("Player Cover: {}", err))
            .ok();
        set_image_from_path(&self.cover_small, show.id(), 34)
            .map_err(|err| error!("Player Cover: {}", err))
            .ok();
    }
}

#[derive(Debug, Clone)]
struct PlayerTimes {
    progressed: gtk::Label,
    duration: gtk::Label,
    slider: gtk::Scale,
    slider_update: Rc<SignalHandlerId>,
    progress_bar: gtk::ProgressBar,
}

#[derive(Debug, Clone, Copy)]
struct Duration(ClockTime);

impl Deref for Duration {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
struct Position(ClockTime);

impl Deref for Position {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerTimes {
    /// Update the duration `gtk::Label` and the max range of the `gtk::SclaeBar`.
    pub(crate) fn on_duration_changed(&self, duration: Duration) {
        let seconds = duration.seconds();

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, seconds as f64);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format_duration(seconds as u32));

        self.update_progress_bar();
    }

    /// Update the `gtk::Scale` bar when the pipeline position is changed.
    pub(crate) fn on_position_updated(&self, position: Position) {
        let seconds = position.seconds();

        self.slider.block_signal(&self.slider_update);
        self.slider.set_value(seconds as f64);
        self.slider.unblock_signal(&self.slider_update);

        self.progressed.set_text(&format_duration(seconds as u32));

        self.update_progress_bar();
    }

    fn update_progress_bar(&self) {
        let fraction = self.slider.value() / self.slider.adjustment().upper();
        self.progress_bar.set_fraction(fraction);
    }
}

fn format_duration(seconds: u32) -> String {
    let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0);
    if time.is_none() {
        return "-".to_string();
    }
    let time = time.unwrap();

    if seconds >= 3600 {
        time.format("%T").to_string()
    } else {
        time.format("%M∶%S").to_string()
    }
}

#[derive(Debug, Clone)]
struct PlayerRate {
    action: gio::SimpleAction,
    btn: gtk::MenuButton,
}

impl PlayerRate {
    fn new() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/player_rate.ui");

        // This needs to be a string to work with GMenuModel
        let variant_type = glib::VariantTy::new("s").expect("Could not parse variant type");
        let action =
            gio::SimpleAction::new_stateful("set", Some(variant_type), &"1.00".to_variant());
        let btn: gtk::MenuButton = builder.object("rate_button").unwrap();

        PlayerRate { action, btn }
    }

    fn connect_signals(&self, widget: &Rc<RefCell<PlayerWidget>>) {
        let group = gio::SimpleActionGroup::new();
        self.action
            .connect_activate(clone!(@weak widget => move |action, rate_v| {
                let variant = rate_v.unwrap();
                action.set_state(variant);
                let rate = variant
                    .get::<String>()
                    .expect("Could not get rate from variant")
                    .parse::<f64>()
                    .expect("Could not parse float from variant string");
                widget.borrow().on_rate_changed(rate);
            }));
        group.add_action(&self.action);
        widget
            .borrow()
            .container
            .insert_action_group("rate", Some(&group));
        widget
            .borrow()
            .dialog
            .dialog
            .insert_action_group("rate", Some(&group));
    }
}

#[derive(Debug, Clone)]
struct PlayerControls {
    play: gtk::Button,
    pause: gtk::Button,
    play_small: gtk::Button,
    pause_small: gtk::Button,
    play_pause_small: gtk::Stack,
    forward: gtk::Button,
    rewind: gtk::Button,
    last_pause: RefCell<Option<DateTime<Local>>>,
}

#[derive(Debug, Clone)]
struct PlayerDialog {
    dialog: gtk::Window,
    cover: gtk::Image,
    play_pause: gtk::Stack,
    play: gtk::Button,
    pause: gtk::Button,
    duration: gtk::Label,
    progressed: gtk::Label,
    slider: gtk::Scale,
    forward: gtk::Button,
    rewind: gtk::Button,
    rate: PlayerRate,
    show: gtk::Label,
    episode: gtk::Label,
}

impl PlayerDialog {
    fn new(rate: PlayerRate) -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/player_dialog.ui");
        let dialog = builder.object("dialog").unwrap();

        let cover = builder.object("cover").unwrap();
        let play_pause = builder.object("play_pause").unwrap();
        let play = builder.object("play").unwrap();
        let pause = builder.object("pause").unwrap();
        let duration = builder.object("duration").unwrap();
        let progressed = builder.object("progressed").unwrap();
        let slider = builder.object("slider").unwrap();
        let rewind = builder.object("rewind").unwrap();
        let forward = builder.object("forward").unwrap();
        let bottom: gtk::Box = builder.object("bottom").unwrap();
        let show = builder.object("show_label").unwrap();
        let episode = builder.object("episode_label").unwrap();

        bottom.prepend(&rate.btn);

        PlayerDialog {
            dialog,
            cover,
            play_pause,
            play,
            pause,
            duration,
            progressed,
            slider,
            forward,
            rewind,
            rate,
            show,
            episode,
        }
    }

    fn initialize_episode(&self, episode: &EpisodeWidgetModel, show: &ShowCoverModel) {
        self.episode.set_text(episode.title());
        self.show.set_text(show.title());

        set_image_from_path(&self.cover, show.id(), 256)
            .map_err(|err| error!("Player Cover: {}", err))
            .ok();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerWidget {
    pub(crate) container: gtk::Box,
    revealer: gtk::Revealer,
    gesture_click: gtk::GestureClick,
    player: gst_player::Player,
    controls: PlayerControls,
    dialog: PlayerDialog,
    full: gtk::Box,
    squeezer: adw::Squeezer,
    timer: PlayerTimes,
    info: PlayerInfo,
    rate: PlayerRate,
    sender: Option<Sender<Action>>,
}

impl Default for PlayerWidget {
    fn default() -> Self {
        let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
        let player = gst_player::Player::new(
            None::<gst_player::PlayerVideoRenderer>,
            // Use the gtk main thread
            Some(dispatcher.upcast::<gst_player::PlayerSignalDispatcher>()),
        );

        // A few podcasts have a video track of the thumbnail, which GStreamer displays in a new
        // window. Make sure it doesn't do that.
        player.set_video_track_enabled(false);

        let mpris = MprisPlayer::new(
            APP_ID.to_string(),
            "GNOME Podcasts".to_string(),
            format!("{}.desktop", APP_ID),
        );
        mpris.set_can_raise(true);
        mpris.set_can_play(false);
        mpris.set_can_seek(false);
        mpris.set_can_set_fullscreen(false);
        mpris.set_can_go_next(false);
        mpris.set_can_go_previous(false);

        let mut config = player.config();
        config.set_user_agent(USER_AGENT);
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();

        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/player_toolbar.ui");

        let play = builder.object("play_button").unwrap();
        let pause = builder.object("pause_button").unwrap();
        let play_small = builder.object("play_button_small").unwrap();
        let pause_small = builder.object("pause_button_small").unwrap();
        let forward: gtk::Button = builder.object("ff_button").unwrap();
        let rewind: gtk::Button = builder.object("rewind_button").unwrap();
        let play_pause_small = builder.object("play_pause_small").unwrap();

        let controls = PlayerControls {
            play,
            pause,
            play_small,
            pause_small,
            play_pause_small,
            forward,
            rewind,
            last_pause: RefCell::new(None),
        };

        let progressed = builder.object("progress_time_label").unwrap();
        let duration = builder.object("total_duration_label").unwrap();
        let slider: gtk::Scale = builder.object("seek").unwrap();
        slider.set_range(0.0, 1.0);
        let player_weak = player.downgrade();
        let slider_update = Rc::new(Self::connect_update_slider(&slider, player_weak));
        let progress_bar = builder.object("progress_bar").unwrap();
        let timer = PlayerTimes {
            progressed,
            duration,
            slider,
            slider_update,
            progress_bar,
        };

        let show = builder.object("show_label").unwrap();
        let episode = builder.object("episode_label").unwrap();
        let cover = builder.object("show_cover").unwrap();
        let show_small = builder.object("show_label_small").unwrap();
        let episode_small = builder.object("episode_label_small").unwrap();
        let cover_small = builder.object("show_cover_small").unwrap();
        let ep = None;
        let info = PlayerInfo {
            mpris,
            show,
            ep,
            episode,
            cover,
            show_small,
            episode_small,
            cover_small,
            restore_position: 0,
            finished_restore: false,
            episode_id: RefCell::new(None),
        };
        info.create_bindings();

        let dialog_rate = PlayerRate::new();
        let dialog = PlayerDialog::new(dialog_rate);

        let container = builder.object("container").unwrap();
        let revealer: gtk::Revealer = builder.object("revealer").unwrap();
        let gesture_click = builder.object("gesture_click").unwrap();
        let full: gtk::Box = builder.object("full").unwrap();
        let squeezer = builder.object("squeezer").unwrap();

        let rate = PlayerRate::new();
        full.append(&rate.btn);

        PlayerWidget {
            player,
            container,
            revealer,
            gesture_click,
            controls,
            dialog,
            full,
            squeezer,
            timer,
            info,
            rate,
            sender: None,
        }
    }
}

impl PlayerWidget {
    fn on_rate_changed(&self, rate: f64) {
        self.set_playback_rate(rate);
        self.rate.btn.set_label(&format!("{:.2}×", rate));
        self.dialog.rate.btn.set_label(&format!("{:.2}×", rate));
    }

    fn reveal(&self) {
        self.revealer.set_reveal_child(true);
    }

    pub(crate) fn initialize_episode(&mut self, rowid: i32, second: Option<i32>) -> Result<()> {
        let ep = dbqueries::get_episode_widget_from_rowid(rowid)?;
        let pd = dbqueries::get_podcast_cover_from_id(ep.show_id())?;

        self.dialog.initialize_episode(&ep, &pd);

        self.info.restore_position = second.unwrap_or(ep.play_position());
        self.info.finished_restore = false;
        self.info.init(&ep, &pd);

        // Currently that will always be the case since the play button is
        // only shown if the file is downloaded
        if let Some(ref path) = ep.local_uri() {
            if Path::new(path).exists() {
                // path is an absolute fs path ex. "foo/bar/baz".
                // Convert it so it will have a "file:///"
                // FIXME: convert it properly
                let uri = File::for_path(path).uri();

                // If it's not the same file load the uri, otherwise just unpause
                if self.player.uri().map_or(true, |s| s != uri.as_str()) {
                    self.player.set_uri(Some(uri.as_str()));
                } else if second.is_some() {
                    // force a jump now if already playing and a jump is given
                    self.restore_play_position();
                } else {
                    // just unpause, no restore required
                    self.info.finished_restore = true;
                }
                // play the file
                self.play();

                return Ok(());
            }
            // TODO: log an error
        }

        // FIXME: Stream stuff
        // unimplemented!()
        Ok(())
    }

    fn connect_update_slider(
        slider: &gtk::Scale,
        player: WeakRef<gst_player::Player>,
    ) -> SignalHandlerId {
        slider.connect_value_changed(move |slider| {
            let player = match player.upgrade() {
                Some(p) => p,
                None => return,
            };

            let value = slider.value() as u64;
            player.seek(ClockTime::from_seconds(value));
        })
    }

    fn smart_rewind(&self) -> Option<()> {
        static LAST_KNOWN_EPISODE: Lazy<Mutex<Option<i32>>> = Lazy::new(|| Mutex::new(None));

        // Figure out the time delta, in seconds, between the last pause and now
        let now = Local::now();
        let last: &Option<DateTime<_>> = &*self.controls.last_pause.borrow();
        let delta = (now - (*last)?).num_seconds();

        // Get interval passed in the gst stream
        let seconds_passed = self.player.position()?.seconds();
        // get the last known episode id
        let mut last = LAST_KNOWN_EPISODE.lock().unwrap();
        // get the current playing episode id
        let current_id = *self.info.episode_id.borrow();
        // Only rewind on pause if the stream position is passed a certain point,
        // and the player has been paused for more than a minute,
        // and the episode id is the same
        if seconds_passed >= 90 && delta >= 60 && current_id == *last {
            self.seek(ClockTime::from_seconds(5), SeekDirection::Backwards);
        }

        // Set the last knows episode to the current one
        *last = current_id;

        Some(())
    }

    /// Seek to the `play_position` stored in the episode.
    /// Returns Some(()) if the restore was successful and None otherwise.
    fn restore_play_position(&self) -> Option<()> {
        let pos = self.info.restore_position;
        let s: u64 = pos.try_into().ok()?;
        if pos != 0 {
            self.player.seek(ClockTime::from_seconds(s));
            Some(())
        } else {
            None
        }
    }
}

impl PlayerExt for PlayerWidget {
    fn play(&self) {
        self.dialog.play_pause.set_visible_child(&self.dialog.pause);

        self.reveal();

        self.controls.pause.set_visible(true);
        self.controls.play.set_visible(false);
        self.controls
            .play_pause_small
            .set_visible_child(&self.controls.pause_small);

        self.smart_rewind();
        self.player.play();
        self.info.mpris.set_playback_status(PlaybackStatus::Playing);
        if let Some(sender) = &self.sender {
            send!(sender, Action::InhibitSuspend);
        }
    }

    fn pause(&mut self) {
        self.dialog.play_pause.set_visible_child(&self.dialog.play);

        self.controls.pause.set_visible(false);
        self.controls.play.set_visible(true);
        self.controls
            .play_pause_small
            .set_visible_child(&self.controls.play_small);

        self.player.pause();
        self.info.mpris.set_playback_status(PlaybackStatus::Paused);
        if let Some(sender) = &self.sender {
            send!(sender, Action::UninhibitSuspend);
        }

        self.controls.last_pause.replace(Some(Local::now()));
        let pos = self.player.position();
        self.info.ep.as_mut().map(|ep| {
            ep.set_play_position(pos.and_then(|s| s.seconds().try_into().ok()).unwrap_or(0))
        });
    }

    fn stop(&mut self) {
        // hide pause buttons and restore focus for accessibility
        let is_focus = self.controls.pause.is_focus();
        self.controls.pause.set_visible(false);
        self.controls.play.set_visible(true);
        if is_focus {
            self.controls.play.grab_focus();
        }

        let is_focus = self.controls.pause_small.is_focus();
        self.controls.pause_small.set_visible(false);
        self.controls.play_small.set_visible(true);
        if is_focus {
            self.controls.play_small.grab_focus();
        }

        let is_focus = self.dialog.pause.is_focus();
        self.dialog.pause.set_visible(false);
        self.dialog.play.set_visible(true);
        if is_focus {
            self.dialog.play.grab_focus();
        }

        self.info.ep = None;
        self.info.restore_position = 0;
        self.player.stop();
        self.info.mpris.set_playback_status(PlaybackStatus::Paused);

        // Reset the slider bar to the start

        self.timer
            .on_position_updated(Position(ClockTime::from_seconds(0)));
        if let Some(sender) = &self.sender {
            send!(sender, Action::UninhibitSuspend);
        }
    }

    // Adapted from https://github.com/philn/glide/blob/b52a65d99daeab0b487f79a0e1ccfad0cd433e22/src/player_context.rs#L219-L245
    fn seek(&self, offset: ClockTime, direction: SeekDirection) -> Option<()> {
        // How far into the podcast we are
        let position = self.player.position()?;
        if offset.is_zero() {
            return Some(());
        }

        // How much podcast we have
        let duration = self.player.duration()?;
        let destination = match direction {
            // If we are more than `offset` into the podcast, jump back that far
            SeekDirection::Backwards if position >= offset => position.checked_sub(offset),
            // If we haven't played `offset` yet just restart the podcast
            SeekDirection::Backwards if position < offset => Some(ClockTime::from_seconds(0)),
            // If we have more than `offset` remaining jump forward they amount
            SeekDirection::Forward if !duration.is_zero() && position + offset <= duration => {
                position.checked_add(offset)
            }
            // We don't have `offset` remaining just move to the end (ending playback)
            SeekDirection::Forward if !duration.is_zero() && position + offset > duration => {
                Some(duration)
            }
            // Who knows what's going on ¯\_(ツ)_/¯
            _ => None,
        };

        // If we calucated a new position, jump to it
        if let Some(destination) = destination {
            self.player.seek(destination)
        }

        Some(())
    }

    fn rewind(&self) {
        let r = self.seek(ClockTime::from_seconds(10), SeekDirection::Backwards);
        if r.is_none() {
            warn!("Failed to rewind");
        }
    }

    fn fast_forward(&self) {
        let r = self.seek(ClockTime::from_seconds(10), SeekDirection::Forward);
        if r.is_none() {
            warn!("Failed to fast-forward");
        }
    }

    fn set_playback_rate(&self, rate: f64) {
        self.player.set_rate(rate);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerWrapper(pub Rc<RefCell<PlayerWidget>>);

impl Default for PlayerWrapper {
    fn default() -> Self {
        PlayerWrapper(Rc::new(RefCell::new(PlayerWidget::default())))
    }
}

impl Deref for PlayerWrapper {
    type Target = Rc<RefCell<PlayerWidget>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerWrapper {
    pub(crate) fn borrow_mut(&self) -> RefMut<'_, PlayerWidget> {
        self.0.borrow_mut()
    }
    pub(crate) fn new(sender: &Sender<Action>) -> Self {
        let w = PlayerWrapper::default();
        w.init(sender);
        w
    }

    fn init(&self, sender: &Sender<Action>) {
        self.borrow_mut().sender = Some(sender.clone());
        self.connect_control_buttons();
        self.connect_rate_buttons();
        self.connect_mpris_buttons(sender);
        self.connect_gst_signals(sender);
        self.connect_dialog();
    }

    fn connect_dialog(&self) {
        let this = self.deref();
        let widget = self.borrow();
        widget
            .squeezer
            .connect_visible_child_notify(clone!(@weak this => move |_| {
                let widget = this.borrow();
                if let Some(child) = widget.squeezer.visible_child() {
                    let full = child == this.borrow().full;
                    if full {
                        this.borrow().revealer.remove_css_class("player-small");
                    } else {
                        this.borrow().revealer.add_css_class("player-small");
                    }
                }
            }));

        widget
            .timer
            .duration
            .bind_property("label", &widget.dialog.duration, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        widget
            .timer
            .progressed
            .bind_property("label", &widget.dialog.progressed, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        widget
            .dialog
            .slider
            .set_adjustment(&widget.timer.slider.adjustment());

        widget
            .gesture_click
            .connect_released(clone!(@weak this => move |_, _, _, _| {
                let this = this.borrow();

                let parent = this.container.root().and_then(|root| {
                    root.downcast::<gtk::Window>().ok()
                }).unwrap();

                info!("showing dialog");
                this.dialog.dialog.set_transient_for(Some(&parent));
                this.dialog.dialog.present();
            }));
    }

    /// Connect the `PlayerControls` buttons to the `PlayerExt` methods.
    fn connect_control_buttons(&self) {
        let this = self.deref();
        let widget = self.borrow();
        // Connect the play button to the gst Player.
        widget
            .controls
            .play
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow().play();
                this.borrow().controls.pause.grab_focus(); // keep focus for accessibility
            }));

        // Connect the pause button to the gst Player.
        widget
            .controls
            .pause
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow_mut().pause();
                this.borrow().controls.play.grab_focus(); // keep focus for accessibility
            }));

        // Connect the play button to the gst Player.
        widget
            .controls
            .play_small
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow().play();
                this.borrow().controls.pause_small.grab_focus(); // keep focus for accessibility
            }));

        // Connect the pause button to the gst Player.
        widget
            .controls
            .pause_small
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow_mut().pause();
                this.borrow().controls.play_small.grab_focus(); // keep focus for accessibility
            }));

        // Connect the rewind button to the gst Player.
        widget
            .controls
            .rewind
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow().rewind();
            }));

        // Connect the fast-forward button to the gst Player.
        widget
            .controls
            .forward
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow().fast_forward();
            }));

        // Connect the play button to the gst Player.
        widget
            .dialog
            .play
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow().play();
                this.borrow().dialog.pause.grab_focus(); // keep focus for accessibility
            }));

        // Connect the pause button to the gst Player.
        widget
            .dialog
            .pause
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow_mut().pause();
                this.borrow().dialog.play.grab_focus(); // keep focus for accessibility
            }));

        // Connect the rewind button to the gst Player.
        widget
            .dialog
            .rewind
            .connect_clicked(clone!(@weak this => move |_| {
                    this.borrow().rewind();
            }));

        // Connect the fast-forward button to the gst Player.
        widget
            .dialog
            .forward
            .connect_clicked(clone!(@weak this => move |_| {
                this.borrow().fast_forward();
            }));
    }

    fn connect_gst_signals(&self, sender: &Sender<Action>) {
        let widget = self.borrow();
        // Log gst warnings.
        widget
            .player
            .connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        // Log gst errors.
        widget
            .player
            .connect_error(clone!(@strong sender => move |_, _error| {
                send!(sender, Action::ErrorNotification(format!("Player Error: {}", _error)));
                let s = i18n("The media player was unable to execute an action.");
                send!(sender, Action::ErrorNotification(s));
            }));

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(Rc::downgrade(self));

        widget
            .player
            .connect_uri_loaded(clone!(@strong weak => move |_, _| {
                if let Some(player_widget) = weak.get().upgrade() {
                    player_widget.borrow().restore_play_position();
                    player_widget.borrow_mut().info.finished_restore = true;
                }
            }));

        // Update the duration label and the slider
        widget
            .player
            .connect_duration_changed(clone!(@strong weak => move |_, clock| {
                if let Some(player_widget) = weak.get().upgrade() {
                    if let Some(c) = clock {
                        player_widget.borrow().timer.on_duration_changed(Duration(c));
                    }
                }
            }));

        // Update the position label and the slider
        widget
            .player
            .connect_position_updated(clone!(@strong weak => move |_, clock| {
                if let Some(player_widget) = weak.get().upgrade() {
                    // write to db
                    if let Some(c) = clock {
                        let pos = Position(c);
                        let finished_restore = player_widget.borrow().info.finished_restore;
                        player_widget.borrow_mut().info.ep.as_mut().map(|ep| {
                            if finished_restore {
                                ep.set_play_position_if_divergent(pos.seconds() as i32)
                            } else {
                                Ok(())
                            }
                        });
                        player_widget.borrow().timer.on_position_updated(pos)
                    }
                }
            }));

        // Reset the slider to 0 and show a play button
        widget
            .player
            .connect_end_of_stream(clone!(@strong sender, @strong weak => move |_| {
                if let Some(player_widget) = weak.get().upgrade() {
                    // write position to db
                    player_widget.borrow_mut().info.ep.as_mut().map(|ep| {
                        ep.set_play_position(0)?;
                        ep.set_played_now()?;
                        send!(sender, Action::RefreshEpisodesViewBGR);
                        send!(sender, Action::RefreshWidgetIfSame(ep.show_id()));
                        let ok : Result<(), podcasts_data::errors::DataError> = Ok(());
                        ok
                    });

                    player_widget.borrow_mut().stop()
                }
            }));
    }

    fn connect_rate_buttons(&self) {
        self.deref().borrow().rate.connect_signals(self.deref());
        self.deref()
            .borrow()
            .dialog
            .rate
            .connect_signals(self.deref());
    }

    fn connect_mpris_buttons(&self, sender: &Sender<Action>) {
        let weak = Rc::downgrade(self);
        let widget = self.borrow();

        // FIXME: Reference cycle with mpris
        let mpris = widget.info.mpris.clone();
        widget
            .info
            .mpris
            .connect_play_pause(clone!(@strong weak => move || {
                let player = match weak.upgrade() {
                    Some(s) => s,
                    None => return
                };

                if let Ok(status) = mpris.get_playback_status() {
                    match status.as_ref() {
                        "Paused" => player.borrow().play(),
                        "Stopped" => player.borrow().play(),
                        _ => player.borrow_mut().pause(),
                    };
                }
            }));
        widget
            .info
            .mpris
            .connect_play(clone!(@strong weak => move || {
                let player = match weak.upgrade() {
                    Some(s) => s,
                    None => return
                };

                player.borrow().play();
            }));

        widget
            .info
            .mpris
            .connect_pause(clone!(@strong weak => move || {
                let player = match weak.upgrade() {
                    Some(s) => s,
                    None => return
                };

                player.borrow_mut().pause();
            }));

        widget
            .info
            .mpris
            .connect_seek(clone!(@strong weak => move |offset: i64| {
                if let Some(p) = weak.upgrade() {
                    let direction = if offset > 0 { SeekDirection::Forward } else { SeekDirection::Backwards };
                    p.borrow().seek(ClockTime::from_useconds(offset.unsigned_abs()), direction);
                }
            }));

        widget
            .info
            .mpris
            .connect_raise(clone!(@strong sender => move || {
                send!(sender, Action::RaiseWindow);
            }));
    }
}
