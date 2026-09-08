#!/bin/sh
set -eu

repo_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
entrypoint="$repo_root/scripts/sops-entrypoint.sh"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/cliptown-sops-entrypoint.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

missing="$scratch/missing.env"
missing_output=$(SOPS_SECRETS_FILE="$missing" "$entrypoint" /bin/sh -c 'printf missing-ok')
[ "$missing_output" = "missing-ok" ]

ciphertext="$scratch/app.env"
printf 'ciphertext fixture\n' >"$ciphertext"
if SOPS_SECRETS_FILE="$ciphertext" SOPS_REQUIRE_KEY=1 \
  "$entrypoint" /bin/sh -c 'exit 0' 2>"$scratch/required-key.err"; then
  echo "entrypoint accepted encrypted input without a required age key" >&2
  exit 1
fi
grep -q 'no SOPS_AGE_KEY or SOPS_AGE_KEY_FILE set' "$scratch/required-key.err"

fake_bin="$scratch/bin"
mkdir "$fake_bin"
cat >"$fake_bin/sops" <<'EOF_SOPS'
#!/bin/sh
cat <<'EOF_ENV'
CLIPTOWN_FROM_SOPS=loaded
CLIPTOWN_WITH_EQUALS=left=right
CLIPTOWN_OVERRIDE=from-sops
bad-key=ignored
sops_mac=ignored
EOF_ENV
EOF_SOPS
chmod 0755 "$fake_bin/sops"

PATH="$fake_bin:/usr/bin:/bin" \
  SOPS_SECRETS_FILE="$ciphertext" \
  SOPS_REQUIRE_KEY=1 \
  SOPS_AGE_KEY='AGE-SECRET-KEY-test-only' \
  CLIPTOWN_OVERRIDE=from-orchestrator \
  "$entrypoint" /usr/bin/env >"$scratch/environment.out" 2>"$scratch/environment.err"

grep -qx 'CLIPTOWN_FROM_SOPS=loaded' "$scratch/environment.out"
grep -qx 'CLIPTOWN_WITH_EQUALS=left=right' "$scratch/environment.out"
grep -qx 'CLIPTOWN_OVERRIDE=from-orchestrator' "$scratch/environment.out"
if grep -q '^bad-key=' "$scratch/environment.out" || grep -q '^sops_mac=' "$scratch/environment.out"; then
  echo "entrypoint exported a rejected ciphertext field" >&2
  exit 1
fi

echo "sops entrypoint tests passed"
