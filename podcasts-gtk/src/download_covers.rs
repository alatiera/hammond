use anyhow::{anyhow, bail, Context, Result};
use glib::WeakRef;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::PathBuf;
use tempdir::TempDir;
use tokio::sync::RwLock; // also works from gtk, unlike tokio::fs

use crate::thumbnail_generator::ThumbSize;
use podcasts_data::errors::DownloadError;
use podcasts_data::errors::DownloadError::NoLongerNeeded;
use podcasts_data::xdg_dirs::CACHED_COVERS_DIR;
use podcasts_data::ShowCoverModel;

// Downloader v3
// if a textures is in the COVER_TEXTURES cache:
//     - return texture form HashMap cache.
// if download lock is set:
//     - sleep for 30 seconds in 250ms intervals
//     - if the lock disapears check if the texture is in cache and return
//     - else try to get a lock for loading it.
//     - if the lock was aquired by another task,
//           sleep for 30 seconds in 25ms intervals
//     - if the lock disapears check if the texture is in cache and return
//     - else bail! and return an error
// if the image is outdated (past the 4 week cache date)
//     - download a copy, then generate thumbnails for it and override the original
// if the image file exists:
//     - load it into cache form fs at the right thumb size and return it
// if the thumb doesn't exist but the file exists:
//     - download a copy, then generate thumbnails for it and override the original
// if the file doesn't exist:
//     - download it, then generate thumbs, cache the requested thumb size
//           and return the texture.

static CACHE_VALID_DURATION: Lazy<chrono::Duration> = Lazy::new(|| chrono::Duration::weeks(4));

type CoverId = (i32, ThumbSize);

// Thumbs that are already loaded
static COVER_TEXTURES: Lazy<RwLock<HashMap<CoverId, gdk::Texture>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
// Each cover should only be downloaded once
static COVER_DL_REGISTRY: Lazy<RwLock<HashSet<i32>>> = Lazy::new(|| RwLock::new(HashSet::new()));
// Each thumb should only be loaded once
static THUMB_LOAD_REGISTRY: Lazy<RwLock<HashSet<CoverId>>> =
    Lazy::new(|| RwLock::new(HashSet::new()));

#[allow(clippy::mutable_key_type)]
fn filename_for_download(response: &reqwest::Response) -> String {
    use reqwest::header::HeaderValue;
    let mime = response.headers().get(reqwest::header::CONTENT_TYPE);

    let headers = HashMap::from([
        (HeaderValue::from_static("image/apng"), ".png"),
        (HeaderValue::from_static("image/avif"), ".avif"),
        (HeaderValue::from_static("image/gif"), ".gif"),
        (HeaderValue::from_static("image/jpeg"), ".jpeg"),
        (HeaderValue::from_static("image/png"), ".png"),
        (HeaderValue::from_static("image/svg"), ".svg"),
        (HeaderValue::from_static("image/webp"), ".webp"),
    ]);

    let mime_extension = mime.and_then(|m| headers.get(m)).unwrap_or(&"");

    // Get filename from url if possible
    let ext = response
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("tmp-donwload.bin");

    if ext.is_empty() {
        return ["tmp-donwload", mime_extension].join("");
    }

    [ext, mime_extension].join("")
}

pub fn clean_unfinished_downloads() -> Result<()> {
    info!("Starting cover locks cleanup");
    let dir = CACHED_COVERS_DIR.clone();

    for entry in std::fs::read_dir(dir)? {
        // keep going if any one file fails
        match entry.map(|e| e.path()) {
            Ok(path) => {
                if let Err(err) = cleanup_entry(&path) {
                    error!("failed to cleanup: {} {err}", path.display());
                }
            }
            Err(err) => error!("failed to get path {err}"),
        }
    }

    Ok(())
}

fn cleanup_entry(path: &PathBuf) -> Result<()> {
    if path.is_file() && path.ends_with(".part") {
        std::fs::remove_file(path)?;
    }
    // remove tmp directories of unfinished downloads
    if path.is_dir() {
        if let Some(filename) = path.to_str() {
            if filename.contains("-pdcover.part") {
                info!("Removing unfinished download: {}", path.display());
                // remove_dir_all can be risky if xdg would break,
                // but we are filtering for a "*-pdcover.part*" dir-name
                // and in a "Covers/" subdir, so it should be fine.
                std::fs::remove_dir_all(path)?;
            }
        }
    }
    Ok(())
}

/// Covers are: XDG_CACHE/Covers/{show_id}
/// Thumbs are: XDG_CACHE/Covers/{show_id}-{size}
/// Also updates (see `determin_cover_path_for_update`)
pub fn determin_cover_path(pd: &ShowCoverModel, size: Option<ThumbSize>) -> PathBuf {
    let mut dir = CACHED_COVERS_DIR.clone();
    let filename = if let Some(size) = size {
        format!("{}-{size}", pd.id())
    } else {
        format!("{}", pd.id())
    };
    dir.push(filename);
    dir
}
/// Updates are: XDG_CACHE/Covers/{show_id}-update
fn determin_cover_path_for_update(pd: &ShowCoverModel) -> PathBuf {
    let mut dir = CACHED_COVERS_DIR.clone();
    let filename = format!("{}-update", pd.id());
    dir.push(filename);
    dir
}

async fn download(
    pd: &ShowCoverModel,
    cover_id: &CoverId,
    path: &PathBuf,
    just_download: bool,
) -> Result<Option<gdk::Texture>> {
    let url = pd
        .image_uri()
        .ok_or(anyhow!("invalid cover uri"))?
        .to_owned();
    if url.is_empty() {
        bail!("No download location");
    }

    let tmp_dir = TempDir::new_in(&*CACHED_COVERS_DIR, &format!("{}-pdcover.part", pd.id()))?;
    let client = podcasts_data::downloader::client_builder().build()?;
    let uri = pd.image_uri().ok_or(anyhow!("No image uri for podcast"))?;
    let response = client.get(uri).send().await?;
    //FIXME: check for 200 or redirects, retry for 5xx
    debug!("Status Resp: {}", response.status());

    let filename = filename_for_download(&response);
    let filename = tmp_dir.path().join(filename);
    info!("Downloading file into: '{:?}'", filename);
    let mut dest = tokio::fs::File::create(&filename).await?;

    let mut content = Cursor::new(response.bytes().await?);
    tokio::io::copy(&mut content, &mut dest).await?;

    dest.sync_all().await?;
    drop(dest);

    // Download done, lets generate thumbnails
    let thumbs = crate::thumbnail_generator::generate(pd, &filename)
        .await
        .context(format!("For {}", filename.display()))?;
    if just_download {
        tokio::fs::rename(&filename, &path).await?;
        return Ok(None);
    }
    if let Some(thumb_texture) = thumbs.get(&cover_id.1) {
        info!("Cached img into: '{}'", &path.display());
        COVER_TEXTURES
            .write()
            .await
            .insert(*cover_id, thumb_texture.clone());
        // Finalize
        // we only rename after thumbnails are generated,
        // so thumbnails can be presumed to exist if the orginal file exists
        tokio::fs::rename(&filename, &path).await?;
        return Ok(Some(thumb_texture.clone()));
    }

    bail!("failed to generate thumbnails");
}

async fn from_web(pd: &ShowCoverModel, cover_id: &CoverId, path: &PathBuf) -> Result<gdk::Texture> {
    // the `false` for just_download gurantees it to be Some
    Ok(download(pd, cover_id, path, false).await?.unwrap())
}

async fn cover_is_downloading(show_id: i32) -> bool {
    COVER_DL_REGISTRY.read().await.contains(&show_id)
}

const SLEEP_TIME: std::time::Duration = std::time::Duration::from_millis(250);
const SLEEP_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);
async fn wait_for_download(pd: &ShowCoverModel, cover_id: &CoverId) -> Result<gdk::Texture> {
    let mut time_waited = std::time::Duration::new(0, 0);
    while {
        // wait for download to finish
        tokio::time::sleep(SLEEP_TIME).await;
        time_waited += SLEEP_TIME;
        if time_waited > SLEEP_LIMIT {
            bail!("Waited too long for download.");
        }
        cover_is_downloading(cover_id.0).await
    } {}
    from_cache_or_fs(pd, cover_id).await
}

async fn from_cache_or_fs(pd: &ShowCoverModel, cover_id: &CoverId) -> Result<gdk::Texture> {
    if let Some(texture) = from_cache(cover_id).await {
        Ok(texture)
    } else {
        // check if someone else is load the thumb
        if THUMB_LOAD_REGISTRY.read().await.contains(cover_id) {
            let mut time_waited = std::time::Duration::new(0, 0);
            while {
                // wait for load to finish
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                time_waited += SLEEP_TIME;
                if time_waited > SLEEP_LIMIT {
                    bail!("Waited too long for thumb read.");
                }
                THUMB_LOAD_REGISTRY.read().await.contains(cover_id)
            } {}
            return from_cache(cover_id)
                .await
                .ok_or(anyhow!("Failed to wait for thumbnail form cache."));
        }
        let got_lock = THUMB_LOAD_REGISTRY.write().await.insert(*cover_id);
        if got_lock {
            let result = from_fs(pd, cover_id).await;
            THUMB_LOAD_REGISTRY.write().await.remove(cover_id);
            result
        } else {
            from_cache(cover_id).await.ok_or(anyhow!(
                "Failed to wait for thumbnail form cache (failed lock)."
            ))
        }
    }
}

async fn from_cache(cover_id: &CoverId) -> Option<gdk::Texture> {
    COVER_TEXTURES.read().await.get(cover_id).cloned()
}

async fn from_fs(pd: &ShowCoverModel, cover_id: &CoverId) -> Result<gdk::Texture> {
    let thumb = determin_cover_path(pd, Some(cover_id.1));
    if let Ok(texture) = gdk::Texture::from_filename(thumb) {
        COVER_TEXTURES
            .write()
            .await
            .insert(*cover_id, texture.clone());
        Ok(texture)
    } else {
        bail!("failed to load texture")
    }
}

async fn from_update(
    pd: &ShowCoverModel,
    cover_id: &CoverId,
    cover: &PathBuf,
) -> Result<gdk::Texture> {
    // Download a potentially updated cover and replace the old.
    // It won't update all images instantly,
    // but that shouldn't be a big problem.
    let update_path = determin_cover_path_for_update(pd);
    let texture = from_web(pd, cover_id, &update_path).await?;
    tokio::fs::rename(&update_path, &cover).await?;
    Ok(texture)
}

async fn aquire_dl_lock(show_id: i32) -> bool {
    COVER_DL_REGISTRY.write().await.insert(show_id)
}
async fn drop_dl_lock(show_id: i32) {
    COVER_DL_REGISTRY.write().await.remove(&show_id);
}

/// Only make sure cover is downloaded without caching any textures.
pub async fn just_download(pd: &ShowCoverModel) -> Result<()> {
    let show_id = pd.id();
    if aquire_dl_lock(show_id).await {
        let cover = determin_cover_path(pd, None);
        // Won't be used because we pass `true` for just_download
        let unused_cover_id = (show_id, crate::Thumb64);
        let result = download(pd, &unused_cover_id, &cover, true).await;
        drop_dl_lock(show_id).await;
        result?;
    } else {
        while {
            // wait for download to finish
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            cover_is_downloading(show_id).await
        } {}
    }
    Ok(())
}
/// Caches and returns the texture, may also download and update it.
pub async fn load_texture(pd: &ShowCoverModel, thumb_size: ThumbSize) -> Result<gdk::Texture> {
    if pd.image_uri().is_none() {
        bail!("no image_uri for this show: {}", pd.title());
    }
    let show_id = pd.id();
    let cover_id = (show_id, thumb_size);
    // early return from memory cache
    if let Some(texture) = from_cache(&cover_id).await {
        return Ok(texture);
    }
    // already loading
    if cover_is_downloading(show_id).await {
        return wait_for_download(pd, &cover_id).await;
    }
    // other task is already loading it.
    if !aquire_dl_lock(show_id).await {
        return wait_for_download(pd, &cover_id).await;
    }
    // check for invalid cache
    if !pd.is_cached_image_valid(&CACHE_VALID_DURATION) {
        let cover = determin_cover_path(pd, None);
        let result = from_update(pd, &cover_id, &cover).await;
        drop_dl_lock(show_id).await;
        return result;
    }
    // load from fs
    if let Ok(texture) = from_fs(pd, &cover_id).await {
        drop_dl_lock(show_id).await;
        return Ok(texture);
    }
    // So isn't downloaded yet or something is broken.
    let cover = determin_cover_path(pd, None);
    let thumb = determin_cover_path(pd, Some(thumb_size));
    let cover_exists = cover.exists();
    // Fallback for if we add more/different thumb sizes,
    // or the user messed with the cache, or the DL was broken (e.g.: html error page).
    if !thumb.exists() && cover_exists {
        warn!(
            "Cover exists, but thumb is missing, Maybe Download was broken. Redownloading Cover!"
        );
        let result = from_update(pd, &cover_id, &cover).await;
        drop_dl_lock(show_id).await;
        return result;
    }
    // load from web
    if !cover_exists {
        info!("Downloading cover: {}", cover.display());
        let result = from_web(pd, &cover_id, &cover).await;
        drop_dl_lock(show_id).await;
        result
    } else {
        drop_dl_lock(show_id).await;
        bail!("The cover exists, but we can't load it?")
    }
}

pub trait TextureWidget {
    fn set_from_texture(&self, texture: &gdk::Texture);
}

impl TextureWidget for gtk::Image {
    fn set_from_texture(&self, texture: &gdk::Texture) {
        self.set_paintable(Some(texture));
    }
}

impl TextureWidget for gtk::Picture {
    fn set_from_texture(&self, texture: &gdk::Texture) {
        self.set_paintable(Some(texture));
    }
}

async fn load_paintable_async<T>(image: &WeakRef<T>, podcast_id: i32, size: ThumbSize) -> Result<()>
where
    T: TextureWidget + IsA<gtk::Widget>,
{
    use podcasts_data::dbqueries;

    let pd = crate::RUNTIME
        .spawn_blocking(move || dbqueries::get_podcast_cover_from_id(podcast_id).unwrap())
        .await?;

    if let Some(image) = image.upgrade() {
        image.set_tooltip_text(Some(pd.title()));
    } else {
        return Err(NoLongerNeeded.into());
    }

    let result = crate::RUNTIME
        .spawn(async move { load_texture(&pd, size).await })
        .await;

    match result {
        Ok(Ok(texture)) => {
            if let Some(image) = image.upgrade() {
                image.set_from_texture(&texture);
                return Ok(());
            }
            Err(NoLongerNeeded.into())
        }
        Ok(Err(err)) => bail!("Failed to load Show Cover: {err}"),
        Err(err) => bail!("Failed to load Show Cover thread-error: {err}"),
    }
}

pub fn load_widget_texture<T>(widget: &T, show_id: i32, size: ThumbSize)
where
    T: TextureWidget + IsA<gtk::Widget>,
{
    // TODO Surface has scale() fn that returns a f64 dpi-scale, maybe use that?
    // TODO maybe load the full size image when bigger than 512 is requested?
    let size = size.hidpi(widget.scale_factor()).unwrap_or(crate::Thumb512);
    let widget = widget.downgrade();
    crate::MAINCONTEXT.spawn_local_with_priority(glib::source::Priority::LOW, async move {
        if let Err(err) = load_paintable_async(&widget, show_id, size).await {
            if let Some(DownloadError::NoLongerNeeded) = err.downcast_ref::<DownloadError>() {
                // weak image reference couldn't be upgraded, no need to print this
                return;
            }
            error!("Failed to load image: {err}");
        }
    });
}
