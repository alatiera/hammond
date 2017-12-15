use gtk;
use gtk::prelude::*;

use hammond_data::Podcast;
use hammond_data::dbqueries;

use views::shows::ShowsPopulated;
use views::empty::EmptyView;

use widgets::show::ShowWidget;
use headerbar::Header;

use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Content {
    pub stack: gtk::Stack,
    pub shows: Rc<ShowStack>,
    episodes: Rc<EpisodeStack>,
}

impl Content {
    pub fn new(header: Rc<Header>) -> Rc<Content> {
        let stack = gtk::Stack::new();
        let shows = ShowStack::new(header);
        let episodes = EpisodeStack::new();

        stack.add_titled(&episodes.stack, "episodes", "Episodes");
        stack.add_titled(&shows.stack, "shows", "Shows");

        Rc::new(Content {
            stack,
            shows,
            episodes,
        })
    }

    pub fn update(&self) {
        self.shows.update();
        self.episodes.update();
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ShowStack {
    pub stack: gtk::Stack,
    header: Rc<Header>,
}

impl ShowStack {
    fn new(header: Rc<Header>) -> Rc<ShowStack> {
        let stack = gtk::Stack::new();

        let show = Rc::new(ShowStack {
            stack,
            header: header.clone(),
        });

        let pop = ShowsPopulated::new_initialized(show.clone(), header);
        let widget = ShowWidget::new();
        let empty = EmptyView::new();

        show.stack.add_named(&pop.container, "podcasts");
        show.stack.add_named(&widget.container, "widget");
        show.stack.add_named(&empty.container, "empty");

        if pop.is_empty() {
            show.stack.set_visible_child_name("empty")
        } else {
            show.stack.set_visible_child_name("podcasts")
        }

        show
    }

    // fn is_empty(&self) -> bool {
    //     self.podcasts.is_empty()
    // }

    pub fn update(&self) {
        self.update_podcasts();
        self.update_widget();
    }

    pub fn update_podcasts(&self) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("podcasts").unwrap();

        let pop = ShowsPopulated::new();
        pop.init(Rc::new(self.clone()), self.header.clone());

        self.stack.remove(&old);
        self.stack.add_named(&pop.container, "podcasts");

        if pop.is_empty() {
            self.stack.set_visible_child_name("empty");
        } else if vis != "empty" {
            self.stack.set_visible_child_name(&vis);
        } else {
            self.stack.set_visible_child_name("podcasts");
        }

        old.destroy();
    }

    pub fn replace_widget(&self, pd: &Podcast) {
        let old = self.stack.get_child_by_name("widget").unwrap();
        let new = ShowWidget::new_initialized(Rc::new(self.clone()), self.header.clone(), pd);

        self.stack.remove(&old);
        self.stack.add_named(&new.container, "widget");
    }

    pub fn update_widget(&self) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("widget").unwrap();

        let id = WidgetExt::get_name(&old).unwrap();
        if id == "GtkBox" {
            return;
        }

        let pd = dbqueries::get_podcast_from_id(id.parse::<i32>().unwrap());
        if let Ok(pd) = pd {
            self.replace_widget(&pd);
            self.stack.set_visible_child_name(&vis);
            old.destroy();
        }
    }

    pub fn switch_podcasts_animated(&self) {
        self.stack
            .set_visible_child_full("podcasts", gtk::StackTransitionType::SlideRight);
    }

    pub fn switch_widget_animated(&self) {
        self.stack
            .set_visible_child_full("widget", gtk::StackTransitionType::SlideLeft)
    }
}

#[derive(Debug, Clone)]
struct RecentEpisodes;

#[derive(Debug, Clone)]
struct EpisodeStack {
    // populated: RecentEpisodes,
    // empty: EmptyView,
    stack: gtk::Stack,
}

impl EpisodeStack {
    fn new() -> Rc<EpisodeStack> {
        let _pop = RecentEpisodes {};
        let empty = EmptyView::new();
        let stack = gtk::Stack::new();

        // stack.add_named(&pop.container, "populated");
        stack.add_named(&empty.container, "empty");
        // FIXME:
        stack.set_visible_child_name("empty");

        Rc::new(EpisodeStack {
            // empty,
            // populated: pop,
            stack,
        })
    }

    fn update(&self) {
        // unimplemented!()
    }
}