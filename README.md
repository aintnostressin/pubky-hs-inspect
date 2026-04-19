# pubky-hs-inspect

CLI tool for inspecting [Pubky](https://pubky.app) homeserver instances.

Resolve PKRR records, query homeserver endpoints, and browse public storage — all from the terminal.

## Features

- **Resolve PKRR records** — look up `_pubky` SVCB/HTTPS DNS records for any public key
- **Inspect homeservers** — discover the homeserver domain or pubkey-as-host for a user
- **Browse public storage** — list and fetch resources from a user's public storage
- **List storage entries** — browse files and directories with type indicators
- **Transport URL resolution** — convert `pubky://` URLs to HTTPS endpoints

## Installation

```bash
cargo install --path .
```

## Usage

```
pubky-hs-inspect [URL] <COMMAND>
```

### Commands

| Command | Description |
|---------|-------------|
| `inspect` | Inspect a homeserver — resolve its PKRR, show metadata and user count |
| `inspect-user` | Inspect a Pubky user — resolve their homeserver, show storage and endpoints |
| `pkdns` | Query raw PKRR DNS records (SVCB/HTTPS) |
| `storage` | List public storage entries for a key |
| `ls` | List files under a path for a user's storage |
| `version` | Show tool version |

### Examples

**Inspect a homeserver** — resolve its PKRR record, show metadata and user count:

```bash
$ pubky-hs-inspect inspect 9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx

═══ Homeserver Inspection ═══

▸ Homeserver Identity
   Input:  9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Z32:    9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx

▸ PKRR Record
   Target:    myhomeserver.pubky.app
   Type:      ICANN domain
   Status:    resolved ✓

▸ Metadata
   Profile URL: https://_pubky.9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx/pub/pubky.app/profile.json
   Users:       42
   Status:      profile fetched ✓
```

**Inspect a Pubky user** — resolves their homeserver, shows storage, and displays transport URLs:

```bash
$ pubky-hs-inspect inspect-user 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty

═══ PKRR Homeserver Inspection ═══

▸ Identity
   Input:  8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Z32:    8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   PKRR Q: _pubky.8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty

▸ PKRR Endpoint Resolution
   Host:      _pubky.8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Status:    PKRR record resolved ✓

▸ Homeserver Resolution
   Query key:   8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Homeserver:  9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Record PK:   9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Status:      resolved ✓
   Base URL:    https://_pubky.9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx/
   Profile:     https://_pubky.9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx/pub/pubky.app/profile.json

▸ Public Storage
   Homeserver: 9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   URL:        https://9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx/pub/
   Found 1 entry(ies):
     pubky://9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx/pub/pubky.app/profile.json
```

**Query raw PKRR DNS records:**

```bash
$ pubky-hs-inspect pkdns 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty

═══ PKRR Record Query ═══

Querying PKRR record: _pubky.8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty

▸ Endpoint Resolution
   Host:    _pubky.8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Domain:  (pubkey-as-host)
```

**List public storage entries:**

```bash
$ pubky-hs-inspect storage 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty

═══ Public Storage Inspector ═══

▸ Homeserver
   Query key:   8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Homeserver:  9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Status:      resolved ✓

▸ Storage Listing
   Total entries: 3

   pubky://<key>/pub/file1.json
   pubky://<key>/pub/file2.txt
   pubky://<key>/pub/pubky.app/profile.json
```

**List files under a storage path:**

```bash
$ pubky-hs-inspect ls 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty

═══ Storage File Listing ═══

▸ Homeserver
   Query key:   8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Homeserver:  9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Status:      resolved ✓

▸ Listing
   Target: pubky://8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty/pub/

   Total entries: 2

   📄 pubky.app/profile.json
   📁 my-app/

$ pubky-hs-inspect ls 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty --path /pub/my-app/

═══ Storage File Listing ═══

▸ Homeserver
   Query key:   8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Homeserver:  9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Status:      resolved ✓

▸ Listing
   Target: pubky://8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty/pub/my-app/

   Total entries: 3

   📄 config.json
   📁 assets/
   📄 index.html

# Navigate into a subdirectory
$ pubky-hs-inspect ls 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty --path /pub/my-app/assets/

═══ Storage File Listing ═══

▸ Listing
   Target: pubky://8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty/pub/my-app/assets/

   Total entries: 4

   📄 logo.png
   📄 style.css
   📄 main.js
   📄 favicon.ico
```

**Resolve a `pubky://` URL:**

```bash
$ pubky-hs-inspect inspect-user pubky<8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty>/pub/file.json
```

## License

MIT