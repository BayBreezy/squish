# Releasing Squish

How to publish a new version that other people can download.

## Publish a new version

1. **Bump the version** in these three files (they must all match; `bun run check:version` checks):
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
2. **Merge it to `main`.**
3. **Tag it and push the tag:**

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

4. **Wait for the build.** GitHub Actions (the **Actions** tab) builds installers for macOS (Apple Silicon and Intel), Windows and Linux and attaches them to a _draft_ release.
5. **Publish it.** Open **Releases**, open the draft, check the files, and press **Publish release**. Share that page.

## Try a build without releasing

**Actions → Release → Run workflow**, pick a branch. The installers appear as downloads on that run. No tag or release is created.

## Warnings when opening the app

The builds are **not code-signed**, so macOS and Windows show a security warning the first time. Users can still open the app: the README explains how. To remove the warning you need to sign the app:

- **macOS:** an Apple Developer account, then add the `APPLE_*` secrets in GitHub (Settings → Secrets and variables → Actions). The workflow already uses them. Guide: <https://v2.tauri.app/distribute/sign/macos/>
- **Windows:** a code-signing certificate. This is not set up in the workflow yet. Guide: <https://v2.tauri.app/distribute/sign/windows/>

## Good to know

- Every pull request and push to `main` runs checks automatically ([`ci.yml`](../.github/workflows/ci.yml)).
- The workflows haven't been run yet, so watch the first release build and expect to fix a small thing or two.
