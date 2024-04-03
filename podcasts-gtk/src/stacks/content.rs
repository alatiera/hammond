// content.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
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

use adw::prelude::*;
use anyhow::Result;
use async_channel::Sender;

use crate::app::Action;
use crate::stacks::{HomeStack, ShowStack};

use std::cell::RefCell;
use std::rc::Rc;

use crate::i18n::i18n;

#[derive(Debug, Clone, Copy)]
pub(crate) enum State {
    Populated,
    Empty,
}

#[derive(Debug, Clone)]
pub(crate) struct Content {
    container: adw::Bin,
    progress_bar: gtk::ProgressBar,
    stack: adw::ViewStack,
    shows: Rc<RefCell<ShowStack>>,
    home: Rc<RefCell<HomeStack>>,
}

impl Content {
    pub(crate) fn new(sender: &Sender<Action>) -> Result<Rc<Content>> {
        let container = adw::Bin::new();
        let stack = adw::ViewStack::new();
        let home = Rc::new(RefCell::new(HomeStack::new(sender.clone())?));
        let shows = Rc::new(RefCell::new(ShowStack::new(sender.clone())));
        let progress_bar = gtk::ProgressBar::new();
        let overlay = gtk::Overlay::new();

        progress_bar.set_valign(gtk::Align::Start);
        progress_bar.set_halign(gtk::Align::Center);
        progress_bar.set_visible(false);
        progress_bar.add_css_class("osd");

        overlay.set_child(Some(&stack));
        overlay.add_overlay(&progress_bar);

        // container will hold the header bar and the content
        container.set_child(Some(&overlay));
        let home_page = stack.add_titled(&home.borrow().get_stack(), Some("home"), &i18n("New"));
        let shows_page =
            stack.add_titled(&shows.borrow().get_stack(), Some("shows"), &i18n("Shows"));

        home_page.set_icon_name(Some("document-open-recent-symbolic"));
        shows_page.set_icon_name(Some("audio-input-microphone-symbolic"));

        let con = Content {
            container,
            progress_bar,
            stack,
            shows,
            home,
        };
        Ok(Rc::new(con))
    }

    pub(crate) fn update(&self) {
        self.update_home();
        self.update_shows_view();
    }

    pub(crate) fn update_home(&self) {
        if let Err(err) = self.home.borrow_mut().update() {
            error!("Failed to update HomeView: {}", err);
        }
    }

    pub(crate) fn update_home_if_background(&self) {
        if self.stack.visible_child_name() != Some("home".into()) {
            self.update_home();
        }
    }

    pub(crate) fn update_shows_view(&self) {
        if let Err(err) = self.shows.borrow_mut().update() {
            error!("Failed to update ShowsView: {}", err);
        }
    }

    pub(crate) fn update_widget_if_same(&self, pid: i32) {
        if let Err(err) = self
            .shows
            .borrow()
            .populated()
            .borrow_mut()
            .update_widget_if_same(pid)
        {
            error!("Failed to update ShowsWidget: {}", err);
        }
    }

    pub(crate) fn get_progress_bar(&self) -> gtk::ProgressBar {
        self.progress_bar.clone()
    }
    pub(crate) fn get_stack(&self) -> adw::ViewStack {
        self.stack.clone()
    }
    pub(crate) fn get_container(&self) -> adw::Bin {
        self.container.clone()
    }

    pub(crate) fn get_shows(&self) -> Rc<RefCell<ShowStack>> {
        self.shows.clone()
    }

    pub(crate) fn go_to_home(&self) {
        self.stack.set_visible_child_name("home");
    }

    pub(crate) fn go_to_shows(&self) {
        self.stack.set_visible_child_name("shows");
    }

    pub(crate) fn switch_to_empty_views(&self) {
        use gtk::StackTransitionType::*;

        self.home
            .borrow_mut()
            .switch_visible(State::Empty, Crossfade);
        self.shows.borrow_mut().switch_visible(State::Empty);
    }

    pub(crate) fn switch_to_populated(&self) {
        use gtk::StackTransitionType::*;

        self.home
            .borrow_mut()
            .switch_visible(State::Populated, Crossfade);
        self.shows.borrow_mut().switch_visible(State::Populated);
    }
}
