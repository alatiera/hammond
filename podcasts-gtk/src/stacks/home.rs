use gtk;
use gtk::prelude::*;
use gtk::StackTransitionType;

use crossbeam_channel::Sender;
use failure::Error;
use podcasts_data::dbqueries::is_episodes_populated;
use podcasts_data::errors::DataError;

use app::Action;
use widgets::{EmptyView, HomeView};

use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
enum State {
    Home,
    Empty,
}

#[derive(Debug, Clone)]
pub(crate) struct HomeStack {
    empty: EmptyView,
    episodes: Rc<HomeView>,
    stack: gtk::Stack,
    state: State,
    sender: Sender<Action>,
}

impl HomeStack {
    pub(crate) fn new(sender: Sender<Action>) -> Result<HomeStack, Error> {
        let episodes = HomeView::new(sender.clone(), None)?;
        let empty = EmptyView::default();
        let stack = gtk::Stack::new();
        let state = State::Empty;

        stack.add_named(episodes.view.container(), "home");
        stack.add_named(empty.deref(), "empty");

        let mut home = HomeStack {
            empty,
            episodes,
            stack,
            state,
            sender,
        };

        home.determine_state()?;
        Ok(home)
    }

    pub(crate) fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub(crate) fn update(&mut self) -> Result<(), Error> {
        self.replace_view()?;
        // Determine the actual state.
        self.determine_state().map_err(From::from)
    }

    fn replace_view(&mut self) -> Result<(), Error> {
        // Get the container of the view
        let old = &self.episodes.view.container().clone();

        // Copy the vertical scrollbar adjustment from the old view.
        let vadj = self.episodes.view.get_vadjustment();
        let eps = HomeView::new(self.sender.clone(), vadj)?;

        // Remove the old widget and add the new one
        // during this the previous view is removed,
        // and the visible child falls back to empty view.
        self.stack.remove(old);
        self.stack.add_named(eps.view.container(), "home");
        // Keep the previous state.
        let s = self.state;
        // Set the visible child back to the previous one to avoid
        // the stack transition animation to show the empty view
        self.switch_visible(s, StackTransitionType::None);

        // replace view in the struct too
        self.episodes = eps;

        // This might not be needed
        old.destroy();

        Ok(())
    }

    fn switch_visible(&mut self, s: State, animation: StackTransitionType) {
        use self::State::*;

        match s {
            Home => {
                self.stack.set_visible_child_full("home", animation);
                self.state = Home;
            }
            Empty => {
                self.stack.set_visible_child_full("empty", animation);
                self.state = Empty;
            }
        }
    }

    fn determine_state(&mut self) -> Result<(), DataError> {
        if is_episodes_populated()? {
            self.switch_visible(State::Home, StackTransitionType::Crossfade);
        } else {
            self.switch_visible(State::Empty, StackTransitionType::Crossfade);
        };

        Ok(())
    }
}
