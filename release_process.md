## Podcasts release process

* Ensure there was a post-release version bump last time
* Update CHANGELOG.md
* Edit appdata.xml with the correct version and release notes
* commit and tag in git
* make a tarball for flathub
* Post-release version bump meson.build


### To make a tarball:

* Do git tag

```
git tag -a '0.4.9' -m '0.4.9'
git push --atomic origin master 0.4.9
```

* Open a Build Terminal in Builder. Shift+Control+Alt+T

```
source /usr/lib/sdk/rust-stable/enable.sh
meson dist
```

### Optional maintenance thingies

- Update flatpak modules
- Run `cargo update`, build and commit the new lockfile.
- Check for [outdated](https://github.com/kbknapp/cargo-outdated) crates `cargo install cargo-outdate && cargo outdated -d 1`

