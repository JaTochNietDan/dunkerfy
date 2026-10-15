# Website deployment

The public landing page is a buildless static site in `website/`, served by Cloudflare Worker `oleary-dunkerfy` at **https://dunkerfy.oleary.club**.

- This repository owns static content and Wrangler deployments.
- The separate `oleary-ops` Terraform repository owns the `cloudflare_workers_domain.sites["dunkerfy"]` custom domain.
- Wrangler deliberately has no routes. Do not create a duplicate DNS record in the general `ops` repository.
- No Coolify, container, GitHub Pages, or GitHub Actions deployment is involved.

## Local preview and checks

```sh
mise install
mise run site-check
mise run site-dev
```

The preview is at `http://localhost:5176`. The sound demo only plays after a click and uses original synthesized tones. It does not distribute macOS system audio or read notifications.

## Publish

Credentials stay outside the repository. In the existing `oleary-ops` directory, mise loads its ignored `.secret.env`:

```sh
cd ../oleary-ops
mise exec -- wrangler deploy --config ../dunkerfy/wrangler.jsonc
```

Alternatively provide a scoped `CLOUDFLARE_API_TOKEN` to `mise run site-deploy` in this repository. Never commit it, put it in `wrangler.jsonc`, or copy it into `website/`.

Deploy the Worker before adding its custom domain. Add the `dunkerfy` hostname/service pair in `oleary-ops/sites.auto.tfvars`, review the complete Terraform plan, and apply only if it changes the intended resource. Preserve all existing domains and DNS. Use that repository’s `scripts/tf.sh` wrapper for state and locking.

Updates to the site need only a Wrangler deploy. For rollback, redeploy the previous Git revision’s `website/` assets; the custom domain does not need to change.

## App releases

Run local tests, lint, and `mise run build`. The build produces a locally signed `dist/Dunkerfy.app`. Package it with the macOS `ditto` utility and publish a GitHub release. The initial release is explicitly marked as a developer preview, with its live-capture and notarization limitations stated on both the website and release page.

Only promote a future build to a verified stable release after live notification testing and the required signing/notarization work. Keep download URLs, version labels, and validation notes in sync.
