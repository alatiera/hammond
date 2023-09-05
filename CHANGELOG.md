# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added:

### Changed:

### Fixed:

### Removed:

## [0.6.0] - 2023-07-04

### Added:
- Support for Dark Mode World/podcasts!199
- Introduce ReadMoreLabel widget World/podcasts!199

### Changed:
- Ported to GTK 4 World/podcasts!199
- Port notifications to AdwToast World/podcasts!199
- Round cover images World/podcasts!199
- Make FileChooser dialogs modal World/podcasts!199
- Port to AdwAboutWindow World/podcasts!219
- Move the update ProgressBar to an Overlay World/podcasts!224
- Switch to Rust 2021 edition World/podcasts!233
- Require GNOME 44 features World/podcasts!233
- Port from deprecated GTK apis World/podcasts!233
- Translation updates

### Fixed:
- Correctly import non-utf8 encoded feeds World/podcasts!214
- Escape markup from show descriptions World/podcasts!199
- Fix action bar style World/podcasts!224
- Use a dedicated tokio runtime instance World/podcasts!234

## [0.5.1] - 2022-01-03

### Added:
- Render lists in episode descriptions World/podcasts!210

### Changed:
- Translation updates

### Fixed:
- Pass a file uri to mpris for the cover art instead of http url World/podcasts!209
- Fix itunes tests World/podcasts!207

## [0.5.0] - 2021-12-04

### Changed:
- Description metadata for app stores World/podcasts!202
- Translation updates

### Fixed:
- Fix the wrong User-Agent header being sent World/podcasts!207
- Make newlines display in episode descriptions display correctly World/podcasts!206

## [0.5.0-beta] - 2021-08-20

### Added:
- View episode descriptions and show notes World/podcasts!178
- Pick up the pace of an episode from where you left off World/podcasts!184
- We now inhibit suspend during playback World/podcasts!188
- Detect Soundcloud playlists and correctly add them as feeds World/podcasts!190
- 0.75 and 0.9 playback rate options World/podcasts!187
- Device form factor and input metadata World/podcasts!195

### Changed:
- Translation updates

### Fixed:
- HTTP authentication when the username might be an email World/podcasts!183
- Correctly set a user agent while downloading in more places World/podcasts!194

### Removed:

## [0.4.9] - 2021-03-11

### Added:
- Automatically refresh Show artwork/covers World/podcasts!176
- Suggest a filename on the export opml dialog World/podcasts!155
- Extract RSS feeds from soundcloud links World/podcasts!177

### Changed:
- Upgrade dependencies World/podcasts!148 World/podcasts!149
- Improve indexing pipeline World/podcasts!150
- Upgrade to `libhandy-1.0` World/podcasts!160 World/podcasts!153
- Use glib channels and refactor actions handling World/podcasts!154
- Use libhandy styling for lists World/podcasts!169
- GTK 3 cleanups in preparation for the GTk 4 port World/podcasts!175
- Translation updates

### Fixed:
- Fix date comparison with episodes from previous year World/podcasts!161
- Fix copying downloads across filesystems World/podcasts!180

### Removed:
- Remove static resources World/podcasts!170

## [0.4.8] - 2020-07-09
### Added:
 - Handy Header/Switcher World/podcasts!130
 - Revealer for long show descriptions World/podcasts!129
 - Adaptive player World/podcasts!131
 - Add 1.75 and 2x playback options World/podcasts!131

### Changed:
 - Use standard macros World/podcasts!138
 - Use async functions and upgrade to futures 0.3 World/podcasts!145
 - Store window size World/podcasts!140
 - Use monospace font for numbers World/podcasts!134
 - Translation updates

### Fixed:
 - Fix phantom window World/podcasts!128
 - <enter> to add feed World/podcasts!130
 - Prevent simultaneous refreshes World/podcasts!133
 - Fix buttons in speed menu World/podcasts!134
 - Internationalisation fixes

## [0.4.7] - 2019-10-23

### Added:
- Improved appdata validation and meson tests World/podcasts!89
- The ability to export show subscriptions to opml files World/podcasts!77
- Support for feeds requiring authentication World/podcasts!120

### Changed:

- Episodes now have a checkmark to show whether or not they've been played World/podcasts!106
- Changed to how errors are shown when adding podcasts World/podcasts!108 World/podcasts!109 World/podcasts!110
- Improved integration of cargo and meson World/podcasts!94
- Refactored some macros for error handling World/podcasts!82
- Refactored the handling of styling changes World/podcasts!119
- Updated the icon to better match the HIG guidelines World/podcasts#102
- Made Podcasts use a GtkApplication subclass World/podcasts!113
- Updated the MPRIS permissions in order to remove a sandbox hole World/podcasts!124
- Bumped gtk and libhandy minimum versions

### Fixed:

- Rewind now works regardless if its the start or the end of the episode World/podcasts!83
- Typos in the README and CONTRIBUTING docs World/podcast!97 World/podcast!98 World/podcast!99 World/podcasts!121
- Show cover is reset properly now if there isn't an image World/podcasts#114
- Query pairs are no longer stripped from URLs World/podcasts!111
- Pause MPRIS button now works on KDE Plasma World/podcasts#115
- The playback widget now properly reflects the playback state on episode change World/podcasts!116

### Removed:

- All preferences World/podcast!104

## [0.4.6] - 2018-10-07

### Added:
- Felix, @haecker-felix, wrote an [mpris crate](https://crates.io/crates/mpris-player) and implemented MPRIS2 client side support! !74 #68

### Changed:
- Download Cancel button was changed to an Icon instead of a label !72
- The applciation will no longer scale below 360p in width 1933c79f7a87d8261d91ca4e14eb51c1ddc66624
- Update to the latest HIG 5050dda4d2f75b706842de8507d115dd5a1bd0a9
- Chris, @brainblasted, upgraded hyper to 0.12, this brings openssl 1.1 support !75
- Pipeline backend is now completely migrated to tokio-runtime 0887789f5e653dd92ad397fb39561df6dffcb45c
- Resume playing an episode will attempt to rewind the track only if more than a minute has passed since the last pause !76

### Fixed:
- Fixed a regression where indexing feeds was blocking the `tokio reactor` #88 !70
- Episodeds Listbox no longer resizes when a download starts #89 !72
- The `total_size` label of the `EpisodeWidget` now behaves correctly if the request fails #90 !73
- The Pipeline will no longer log things in stderr for Requests that returned 304 and are expected to be skipped da361d0cb93cd8edd076859b2c607509a96dac8d
- A bug where the HomeView wold get into an invalid state if your only shows had no episodes 32bd2a89a34e8e940b3b260c6be76defe11835ed

### Translations:

**Added**
- Brazilian Portuguese translation 586cf16f
- Swedish translation 2e527250
- Italian translation a23297e5
- Friulian translation 60e09c0d
- Hungarian translation 2751a828
- Croatian translation 0476b67b
- Latvian translation a681b2c9
- Czech translation 3563a964
- Catalan translation 6ea3fc91

**Updated**
- German translation
- Finnish translation
- Polish translation
- Turkish translation
- Croatian translation
- Indonesian translation
- Spanish translation


## [0.4.5] - 2018-08-31

### Added:
- [OARS](https://hughsie.github.io/oars/) Tags where added for compatibility with Store clients b0c94dd9
- Daniel added support for Translations !46
- Svitozar Cherepii(@svito) created a [wiki page](https://wiki.gnome.org/Apps/Podcasts) 70e79e50
- Libhandy was added as a dependency #70
- Development builds can now be installed in parallel with stable builds !64

### Changed:
- The update indication was moved to an In-App notification #72
- The app icon's accent color was changed from orange to red 0dfb4859
- The stack switcher in the Headerbar is now insensitive on Empty Views !63

### Fixed:
- Improved handling of HTTP redirections #64 !61 !62
- Fixed a major performance regression when loading show covers !67
- More reference cycles have been fixed !59
- OPML import dialog now exits properly and no longer keeps the application from shutting down !65
- Update action is disabled if there isn't something to update #71

### Translations:
- Added Finish 93696026
- Added Polish 1bd6efc0
- Added Turkish 73929f2d
- Added Spanish !46
- Added German 6b6c390c
- Added Galician 0060a634
- Added Indonesian ded0224f
- Added Korean 36f16963


## [0.4.4] - 2018-07-31

### Changed:
- `SendCell` crate was replaced with `Fragile`. (Jorda Petridis) 838320785ebbea94e009698b473495cfec076f54
- Update dependencies (Jorda Petridis) 91bea8551998b16e44e5358fdd43c53422bcc6f3

### Fixed:
- Fix more reference cycles. (Jorda Petridis) 3496df24f8d8bfa8c8a53d8f00262d42ee39b41c
- Actually fix cargo-vendor (Jorda Petridis)

## [0.4.3] - 2018-07-27

### Fixed:

- Fix the cargo vendor config for the tarball releash script. (Jorda Petridis) a2440c19e11ca4dcdbcb67cd85259a41fe3754d6

## [0.4.2] - 2018-07-27

### Changed:

- Minimum size requested by the Views. (Jorda Petridis) 7c96152f3f53f271247230dccf1c9cd5947b685f

### Fixed:

- Screenshot metadata in appstream data. (Jorda Petridis) a2440c19e11ca4dcdbcb67cd85259a41fe3754d6

## [0.4.1] - 2018-07-26
### Added:

- Custom icons for the fast-forward and rewind actions in the Player were added. (Tobias Bernard) e77000076b3d78b8625f4c7ef367376d0130ece6
- Hicolor and symbolic icons for the Application. (Tobias Bernard and Sam Hewitt) edae1b04801dba9d91d5d4145db79b287f0eec2c
- Basic prefferences dialog (Zander Brown). [34](https://gitlab.gnome.org/World/podcasts/merge_requests/34)
- Dbus service preparation. Not used till the MPRIS2 integration has landed. (Zander Brown) [42](https://gitlab.gnome.org/World/podcasts/merge_requests/42)
- Episodes and Images will only get drawn when needed. Big Performance impact. (Jordan Petridis) [43](https://gitlab.gnome.org/World/podcasts/merge_requests/43)

### Changed:

- The `ShowWidget` control button were moved to a secondary menu in the Headerbar. (Jordan Petridis) 536805791e336a3e112799be554706bb804d2bef
- EmptyView layout improvements. (Jorda Petridis) 3c3d6c1e7f15b88308a9054b15a6ca0d8fa233ce 518ea9c8b57885c44bda9c418b19fef26ae0e55d
- Improved the `AddButton` behavior. (Jorda Petridis) 67ab54f8203f19aad198dc49e935127d25432b41

### Fixed:

- A couple reffence cycles where fixed. (Jorda Petridis)

### Removed:

- The delay between the application startup and the `update_on_startup` action. (Jorda Petridis) 7569465a612ee5ef84d0e58f4e1010c8d14080d4

## [0.4.0] - 2018-07-04
### Added:
- Keyboard Shortcuts and a Shortcuts dialog were implemented. (ZanderBrown)
[!33](https://gitlab.gnome.org/World/podcasts/merge_requests/33)

### Changed:
- The `FileChooser` of the OPML import was changed to use the `FileChooserNative` widget/API. (ZanderBrown)
[!33](https://gitlab.gnome.org/World/podcasts/merge_requests/33)
- The `EpisdeWidget` was refactored.
[!38](https://gitlab.gnome.org/World/podcasts/merge_requests/38)
- `EpisdeWidget`'s progressbar was changed to be non-blocking and should feel way more responsive now. 9b0ac5b83dadecdff51cd398293afdf0d5276012
- An embedded audio player was implemented!
[!40](https://gitlab.gnome.org/World/podcasts/merge_requests/40)
- Various Database changes.
[!41](https://gitlab.gnome.org/World/podcasts/merge_requests/41)

### Fixed:
- Fixed a bug where the about dialog would be unclosable. (ZanderBrown) [!37](https://gitlab.gnome.org/World/podcasts/merge_requests/37)

## [0.3.4] - 2018-05-20
### Fixed:
- Flatpak can now access the Home folder. This fixes the OPML import feature from
not being able to access any file.

## [0.3.3] - 2018-05-19
### Added:
- Initial functionality for importing shows from an OPML file was implemented.
- ShowsView now rembmers the vertical alignment of the scrollbar between refreshes. 4d2b64e79d8518454b3677612664cd32044cf837

### Changed:
- Minimum `rustc` version requirement was bumped to `1.26`
- Some animations should be smoother now. 7d598bb1d08b05fd5ab532657acdad967c0afbc3
- InAppNotification now can be used to propagate some errors to the user. 7035fe05c4741b3e7ccce6827f72766226d5fc0a and 118dac5a1ab79c0b4ebe78e88256a4a38b138c04

### Fixed:
- Fixed a of by one bug in the `ShowsView` where the last show was never shown. bd12b09cbc8132fd39a266fd091e24bc6c3c040f

## [0.3.2] - 2018-05-07
### Added:
- Vies now have a new fancy scrolling animation when they are refereshed.

### Changed:
- Downloading and loading images now is done asynchronously and is not blocking programs execution.
[#7](https://gitlab.gnome.org/World/podcasts/issues/7)
- Bold, italics links and some other `html` tags can now be rendered in the Show Description.
[#25](https://gitlab.gnome.org/World/podcasts/issues/25)
- `Rayon` Threadpools are now used instead of unlimited one-off threads.
- `EpisdeWidget`s are now loaded asynchronously across views.
- `EpisodeWidget`s no longer trigger a `View` refresh for trivial stuff 03bd95184808ccab3e0ea0e3713a52ee6b7c9ab4
- `ShowWidget` layout was changed 9a5cc1595d982f3232ee7595b83b6512ac8f6c88
- `ShowWidget` Description is inside a scrolled window now

### Fixed:
- `EpisodeWidget` Height now is consistent across views [#57](https://gitlab.gnome.org/World/podcasts/issues/57)
- Implemented a tail-recursion loop to follow-up when a feed redirects to another url. c6a24e839a8ba77d09673f299cfc1e64ba7078f3

### Removed:
- Removed the custom configuration file and replaced instructions to just use meson. 1f1d4af8ba7db8f56435d13a1c191ecff3d4a85b

## [0.3.1] - 2018-03-28
### Added:
- Ability to mark all episodes of a Show as watched.
[#47](https://gitlab.gnome.org/World/podcasts/issues/47)
- Now you are able to subscribe to itunes™ podcasts by using the itunes link of the show.
[#49](https://gitlab.gnome.org/World/podcasts/issues/49)
- Hammond now remembers the window size and position. (Rowan Lewis)
[#50](https://gitlab.gnome.org/World/podcasts/issues/50)
- Implemnted the initial work for integrating with GSettings and storing preferences. (Rowan Lewis)
[!22](https://gitlab.gnome.org/World/podcasts/merge_requests/22) [!23](https://gitlab.gnome.org/World/podcasts/merge_requests/23)
- Shows without episodes now display an empty message similar to EmptyView.
[#44](https://gitlab.gnome.org/World/podcasts/issues/44)

### Changed:
- EpisdeWidget has been reimplemented as a compile time state machine.
[!18](https://gitlab.gnome.org/World/podcasts/merge_requests/18)
- Content Views no longer scroll horizontally when shrunk below their minimum size.
[#35](https://gitlab.gnome.org/World/podcasts/issues/35)
- Some requests now use the Tor Browser's user agent. (Rowan Lewis)
[#53](https://gitlab.gnome.org/World/podcasts/issues/53)

### Fixed:
- Double border aroun the main window was fixed. (Rowan Lewis)
[#52](https://gitlab.gnome.org/World/podcasts/issues/52)

## [0.3.0] - 2018-02-11
- Tobias Bernard Redesigned the whole Gtk+ client.
- Complete re-write of hammond-data and hammond-gtk modules.
- Error handling for all crates was migrated from error-chain to Failure.
- Hammond-data now uses futures to parse feeds.
- Custom gtk-widgets are now composed structs as opposed to functions returning Gtk widgets.

## [0.2.0] - 2017-11-28
- Database Schema Breaking Changes.
- Added url sanitization. #4.
- Reworked and refactored of the hammond-data API.
- Added some more unit tests
- Documented hammond-data public API.

## [0.1.1] - 2017-11-13
- Added appdata.xml file

## [0.1.0] - 2017-11-13
- Initial Release
