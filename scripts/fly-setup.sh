#!/usr/bin/env bash
# One-time setup for langbank.dev. Run the steps in order; each says what it
# needs before it will work. Nothing here is idempotent-hostile — re-running a
# step that already succeeded is safe.
#
# Two facts drive the whole thing:
#   * the preview workflow CREATES an app per pull request, so its token must be
#     ORG-scoped. An app-scoped deploy token deploys production and fails every
#     preview with a confusing auth error.
#   * `fly certs` needs the app to exist, so the first deploy comes before DNS.
set -euo pipefail

ORG="${FLY_ORG:-personal}"        # override: FLY_ORG=powderworks scripts/fly-setup.sh
APP=langbank
REPO=PowderworksCode/langbank

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

step "1. flyctl"
if ! command -v flyctl >/dev/null; then
  curl -L https://fly.io/install.sh | sh
  export FLYCTL_INSTALL="${FLYCTL_INSTALL:-$HOME/.fly}"
  export PATH="$FLYCTL_INSTALL/bin:$PATH"
  # $PATH must stay literal: this line is text for the user's shell profile.
  # shellcheck disable=SC2016
  echo 'add to your shell profile: export PATH="$HOME/.fly/bin:$PATH"'
fi
flyctl version

step "2. log in  (interactive — opens a browser)"
flyctl auth whoami || flyctl auth login

step "3. create the app"
# deploy.yml would create this itself, but it has to exist now so that step 6
# can read its IPs before the domain is pointed anywhere.
flyctl status --app "$APP" >/dev/null 2>&1 \
  || flyctl apps create "$APP" --org "$ORG"

step "4. an ORG-scoped token that can create apps"
# NOT `fly tokens create deploy -a langbank` — that one cannot create the
# per-PR preview apps, and every preview would fail on auth.
TOKEN=$(flyctl tokens create org "$ORG" --name langbank-ci --expiry 8760h)
gh secret set FLY_PREVIEW_TOKEN --repo "$REPO" --body "$TOKEN"
gh variable set FLY_ORG --repo "$REPO" --body "$ORG"
gh secret list --repo "$REPO"
gh variable list --repo "$REPO"

step "5. first deploy"
# Or just merge the PR — deploy.yml does exactly this on push to main.
flyctl deploy --app "$APP" --config fly.toml --dockerfile Dockerfile --remote-only
curl -fsS "https://$APP.fly.dev/health"; echo

step "6. the DNS records to add at your registrar"
flyctl ips list --app "$APP"
cat <<'DNS'

  Add these at whoever holds langbank.dev:

    A     @      <the v4 address above>
    AAAA  @      <the v6 address above>
    CNAME www    langbank.fly.dev

DNS
read -r -p "press enter once DNS is saved (propagation can take a few minutes) "

step "7. certificates"
flyctl certs create langbank.dev --app "$APP"
flyctl certs create www.langbank.dev --app "$APP"
flyctl certs check langbank.dev --app "$APP"

step "8. done — verify"
curl -fsS https://langbank.dev/health; echo
echo "if that 404s or hangs, the cert is still issuing; \`flyctl certs check\` says which."
