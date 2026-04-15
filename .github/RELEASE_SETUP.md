# Release setup — one-time

The [release workflow](workflows/release.yml) builds signed + notarized DMGs for both
Apple Silicon and Intel, then publishes them as a GitHub Release draft whenever you push
a `v*` tag.

## 1. Export your Developer ID cert as .p12

Keychain Access → "login" keychain → filter by "My Certificates" → right-click
**Developer ID Application: <YOUR_NAME> (<TEAM_ID>)** → Export… → save as
`bishop-signing.p12` with a password you'll remember.

## 2. Base64-encode the .p12 for GitHub

```sh
base64 -i bishop-signing.p12 | pbcopy
```

The base64 string is now on your clipboard.

## 3. Add the secrets

GitHub → repo → Settings → Secrets and variables → Actions → New repository secret.
Add each of these:

| Secret                        | Value                                                         |
| ----------------------------- | ------------------------------------------------------------- |
| `APPLE_CERTIFICATE`           | Paste the base64 string from step 2.                          |
| `APPLE_CERTIFICATE_PASSWORD`  | The password you set when exporting the .p12.                 |
| `APPLE_SIGNING_IDENTITY`      | `Developer ID Application: <YOUR_NAME> (<TEAM_ID>)`       |
| `APPLE_ID`                    | Your Apple developer-account email.                           |
| `APPLE_PASSWORD`              | App-specific password from appleid.apple.com.                 |
| `APPLE_TEAM_ID`               | `<TEAM_ID>`                                                  |
| `KEYCHAIN_PASSWORD`           | Any random string — used only for the temp runner keychain.   |

## 4. Cut a release

```sh
git tag v0.1.1
git push origin v0.1.1
```

The workflow kicks off, builds both architectures (~10–15 min), and opens a **draft**
release with both DMGs attached. Review it on GitHub → Releases and click Publish.

## 5. Delete the local .p12

Once the secrets are set, delete `bishop-signing.p12` from disk. The cert still lives in
your Keychain for local signed builds via `./scripts/build-signed.sh`.
