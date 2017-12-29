use send_cell::SendCell;
use glib;
use gdk_pixbuf::Pixbuf;

use hammond_data::feed;
use hammond_data::{PodcastCoverQuery, Source};
use hammond_downloader::downloader;

use std::thread;
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;
use std::rc::Rc;
use std::collections::HashMap;

use content::Content;
use headerbar::Header;

type Foo = RefCell<Option<(Rc<Content>, Rc<Header>, Receiver<bool>)>>;

// Create a thread local storage that will store the arguments to be transfered.
thread_local!(static GLOBAL: Foo = RefCell::new(None));

/// Update the rss feed(s) originating from `Source`.
/// If `source` is None, Fetches all the `Source` entries in the database and updates them.
/// `delay` represents the desired time in seconds for the thread to sleep before executing.
/// When It's done,it queues up a `podcast_view` refresh.
pub fn refresh_feed(content: Rc<Content>, headerbar: Rc<Header>, source: Option<Vec<Source>>) {
    headerbar.show_update_notification();

    // Create a async channel.
    let (sender, receiver) = channel();

    // Pass the desired arguments into the Local Thread Storage.
    GLOBAL.with(clone!(content, headerbar => move |global| {
        *global.borrow_mut() = Some((content.clone(), headerbar.clone(), receiver));
    }));

    thread::spawn(move || {
        if let Some(s) = source {
            feed::index_loop(s);
        } else {
            let e = feed::index_all();
            if let Err(err) = e {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", err);
            };
        };

        sender.send(true).expect("Couldn't send data to channel");;
        glib::idle_add(refresh_everything);
    });
}

fn refresh_everything() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref content, ref headerbar, ref reciever)) = *global.borrow() {
            if reciever.try_recv().is_ok() {
                content.update();
                headerbar.hide_update_notification();
            }
        }
    });
    glib::Continue(false)
}

lazy_static! {
    static ref CACHED_PIXBUFS: Mutex<HashMap<(i32, u32), Mutex<SendCell<Pixbuf>>>> = {
        Mutex::new(HashMap::new())
    };
}

// Since gdk_pixbuf::Pixbuf is refference counted and every episode,
// use the cover of the Podcast Feed/Show, We can only create a Pixbuf
// cover per show and pass around the Rc pointer.
//
// GObjects do not implement Send trait, so SendCell is a way around that.
// Also lazy_static requires Sync trait, so that's what the mutexes are.
// TODO: maybe use something that would just scale to requested size?
pub fn get_pixbuf_from_path(pd: &PodcastCoverQuery, size: u32) -> Option<Pixbuf> {
    let mut hashmap = CACHED_PIXBUFS.lock().unwrap();
    {
        let res = hashmap.get(&(pd.id(), size));
        if let Some(px) = res {
            let m = px.lock().unwrap();
            return Some(m.clone().into_inner());
        }
    }

    let img_path = downloader::cache_image(pd)?;
    let px = Pixbuf::new_from_file_at_scale(&img_path, size as i32, size as i32, true).ok();
    if let Some(px) = px {
        hashmap.insert((pd.id(), size), Mutex::new(SendCell::new(px.clone())));
        return Some(px);
    }
    None
}

#[cfg(test)]
mod tests {
    use hammond_data::Source;
    use hammond_data::feed::index;
    use hammond_data::dbqueries;
    use diesel::associations::Identifiable;
    use super::*;

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/hammond/hammond.db` so we make it explicit
    // to run it.
    #[ignore]
    fn test_get_pixbuf_from_path() {
        let url = "http://www.newrustacean.com/feed.xml";

        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id().clone();

        // Convert Source it into a Feed and index it
        let feed = source.into_feed(true).unwrap();
        index(&feed);

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap();
        let pxbuf = get_pixbuf_from_path(&pd.into(), 256);
        assert!(pxbuf.is_some());
    }
}
