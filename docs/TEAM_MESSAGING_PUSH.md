# Team Messaging Push Setup

Team messages work without APNs credentials. When credentials are missing, the server stores unread counts and the iOS app falls back to unread polling/local notifications while it is allowed to run.

For real remote push delivery, configure these environment variables before starting the server:

- `APNS_KEY_ID`: Apple push notification key ID.
- `APNS_TEAM_ID`: Apple developer team ID.
- `APNS_BUNDLE_ID`: iOS app bundle ID, currently `test.CabNet` unless changed in Xcode.
- `APNS_PRIVATE_KEY_PATH`: path to the `.p8` APNs auth key.
- `APNS_PRIVATE_KEY`: optional inline `.p8` key value. Use this instead of `APNS_PRIVATE_KEY_PATH` only when the hosting environment cannot mount a key file. Escaped `\n` line breaks are supported.

The iOS app registers device tokens at `POST /api/devices/push-tokens`. The server delivers APNs notifications after a Team message is saved, records delivery attempts in `push_delivery_log`, and disables invalid/unregistered tokens when APNs rejects them.

The app target includes `CabNet.entitlements` with `aps-environment` set to `development`. Before production/TestFlight distribution, verify the bundle ID and signing profile have Push Notifications enabled and update the entitlement/environment as needed.