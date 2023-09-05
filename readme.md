# catalyst

(wip) flexible social blogging/news platform for encouraging intercommunity communication among local communities.

### building

build webserver:
`cargo build --release`
build wasm blob:
`cargo run --bin xtask`

### running

`cargo run --release --bin catalyst`

This will run a webserver on 0.0.0.0:8080 (or localhost:8080).

## Roadmap

In intended order of implementation:

 - [x] Make and reference immutable posts with sanitized html
  - [ ] Support browsers that don't have setHTML support (?)
 - [x] Special posts that allow linking and post management/"deletion"
 - [x] Optionally sign posts and maintain an identity
 - [ ] Document API endpoints and expected3
 - [ ] Support multiple identities
 - [ ] Author profile pages
 - [ ] Sync posts between nodes via webrtc
  - [ ] Per user and per instance permissioning
 - [ ] Use webrtc for user-facing chat+video too
 - [ ] Harden privacy and security guarantees
  - [ ] Document trust model clearly
  - [ ] Private/encrypted/group-private posts & messages
 - [ ] Basic image+video transcoding, hosting
 - [ ] Identity/fact attestation and local user beliefs. 
 - [ ] Claim+verify same-person-ness (for multidevice support)

 In no particular order:

 - [ ] Better optimize DB queries for memory use
  - [ ] some system to create and execute db migrations
 - [ ] User friendly post editing (automate creating a new post linked to an outdated post)
 - [ ] WYSIWYG editor mode and tools
 - [ ] First class listings/classifieds post types to let folks do commercial stuff easily
  - [ ] Enable isters to use their preferred payment backends safely
  - [ ] Special post types for IOUs and other public commitments
 - [ ] Give users access to directly modify local recommendation engine
 - [ ] Search / Topics
 - [ ] Notifications of some kind (not push), read notifications tracking
