use anyhow::{anyhow, bail, Result};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tempdir::TempDir;
use tokio::sync::RwLock; // also works from gtk, unlike tokio::fs

use gio::Cancellable;
use glib::WeakRef;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use crate::thumbnail_generator::ThumbSize;
// use podcasts_data::errors::DownloadError;
use podcasts_data::xdg_dirs::CACHED_COVERS_DIR;
use podcasts_data::ShowCoverModel;

// Downloader v3
// if a textures is in the COVER_TEXTURES cache:
//     - Set the image to that texture.
// if file doesn't exist:
//     - Create 0byte placeholder file.
//     - Start Download into tmp file.
//     - Generate thumbnails.
//     - Move the download to the final path.
//     - goto: (if file exists) ↓
// if 0byte exits:
//     - Create FileMonitor
//     - Register load callback on changed
//     - Return the monitor for a widget to keep around until it sets the cover.
//       TODO: drop the monitor after that happens
// if file exists:
//     - check COVER_TEXTURES cache, set image from it if any hits, else
//     - Load the file's texture into the COVER_TEXTURES cache.
//     - set the image to the texture
//     - Only this needs the gtk widget, rest can be done off thread

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

fn filename_for_download(response: &reqwest::Response) -> &str {
    // Get filename from url if possible
    let ext = response
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("tmp-donwload.bin");

    if ext.is_empty() {
        return "tmp-donwload.bin";
    }

    ext
}

pub fn clean_unfinished_downloads() -> Result<()> {
    info!("Starting cover locks cleanup");
    let dir = CACHED_COVERS_DIR.clone();

    for entry in fs::read_dir(dir)? {
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
    if path.is_file() {
        if path.ends_with(".part") {
            fs::remove_file(&path)?;
        }
    }
    // remove tmp directories of unfinished downloads
    if path.is_dir() {
        if let Some(filename) = path.to_str() {
            if filename.contains("-pdcover.part") {
                info!("Removing unfinished download: {}", path.display());
                // remove_dir_all can be risky if xdg would break,
                // but we are filtering for a "*-pdcover.part*" dir-name
                // and in a "Covers/" subdir, so it should be fine.
                fs::remove_dir_all(&path)?;
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

async fn from_web(pd: &ShowCoverModel, cover_id: &CoverId, path: &PathBuf) -> Result<gdk::Texture> {
    let url = pd
        .image_uri()
        .ok_or(anyhow!("invalid cover uri"))?
        .to_owned();
    if url.is_empty() {
        bail!("No download location");
    }

    let tmp_dir = TempDir::new_in(&*CACHED_COVERS_DIR, &format!("{}-pdcover.part", pd.id()))?;
    let client = podcasts_data::downloader::client_builder().build()?;
    let response = client.get(pd.image_uri().unwrap()).send().await?;
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
    let texture = gdk::Texture::from_filename(&filename)?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let pd = pd.clone();
    crate::MAINCONTEXT.spawn_with_priority(glib::source::Priority::DEFAULT_IDLE, async move {
        let thumbs = crate::thumbnail_generator::generate(&pd, texture).await;
        let _ = sender.send(thumbs);
    });

    if let Ok(Ok(thumbs)) = receiver.await {
        if let Some(thumb_texture) = thumbs.get(&cover_id.1) {
            info!("Cached img into: '{}'", &path.display());
            COVER_TEXTURES
                .write()
                .await
                .insert(cover_id.clone(), thumb_texture.clone());
            // Finalize
            // we only rename after thumbnails are generated,
            // so thumbnails can be presumed to exist if the orginal file exists
            tokio::fs::rename(&filename, &path).await?;
            return Ok(thumb_texture.clone());
        }
    }
    bail!("failed to generate thumbnails");
}

async fn cover_is_downloading(show_id: i32) -> bool {
    COVER_DL_REGISTRY.read().await.contains(&show_id)
}

async fn wait_for_download(pd: &ShowCoverModel, cover_id: &CoverId) -> Result<gdk::Texture> {
    while {
        // wait for download to finish
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        cover_is_downloading(cover_id.0).await
    } {}
    return from_cache_or_fs(pd, cover_id).await;
}

async fn from_cache_or_fs(pd: &ShowCoverModel, cover_id: &CoverId) -> Result<gdk::Texture> {
    if let Some(texture) = from_cache(cover_id).await {
        Ok(texture)
    } else {
        // check if someone else is load the thumb
        if THUMB_LOAD_REGISTRY.read().await.contains(cover_id) {
            while {
                // wait for load to finish
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                THUMB_LOAD_REGISTRY.read().await.contains(cover_id)
            } {}
            return from_cache(cover_id)
                .await
                .ok_or(anyhow!("Failed to wait for thumbnail form cache."));
        }
        let got_lock = THUMB_LOAD_REGISTRY.write().await.insert(cover_id.clone());
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
    let thumb = determin_cover_path(pd, Some(cover_id.1.clone()));
    if let Ok(texture) = gdk::Texture::from_filename(thumb) {
        COVER_TEXTURES
            .write()
            .await
            .insert(cover_id.clone(), texture.clone());
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
async fn delete_and_redownload(
    pd: &ShowCoverModel,
    cover_id: &CoverId,
    target_path: &PathBuf,
) -> Result<gdk::Texture> {
    tokio::fs::remove_file(&target_path).await?;
    from_web(pd, cover_id, &target_path).await
}

async fn aquire_dl_lock(show_id: i32) -> bool {
    COVER_DL_REGISTRY.write().await.insert(show_id)
}
async fn drop_dl_lock(show_id: i32) {
    COVER_DL_REGISTRY.write().await.remove(&show_id);
}

pub async fn load_texture(pd: &ShowCoverModel, thumb_size: ThumbSize) -> Result<gdk::Texture> {
    let show_id = pd.id();
    let cover_id = (show_id, thumb_size.clone());
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
        let result = delete_and_redownload(pd, &cover_id, &cover).await;
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

pub async fn load_image(image: &WeakRef<gtk::Image>, podcast_id: i32, size: ThumbSize) {
    use podcasts_data::dbqueries;
    let result = crate::RUNTIME
        .spawn(async move {
            let pd = crate::RUNTIME
                .spawn_blocking(move || dbqueries::get_podcast_cover_from_id(podcast_id).unwrap())
                .await?;
            load_texture(&pd, size).await.map(|t| (pd, t))
        })
        .await;

    match result {
        Ok(Ok((pd, texture))) => {
            if let Some(image) = image.upgrade() {
                image.set_tooltip_text(Some(pd.title()));
                image.set_paintable(Some(&texture));
            }
        }
        Ok(Err(err)) => error!("Failed to load Show Cover: {err}"),
        Err(err) => error!("Failed to load Show Cover thread-error: {err}"),
    }
}
