#!/usr/bin/env bash
set -euo pipefail

# This guard runs on pull requests and enforces that architecture/development
# history in docs/evolution is updated whenever code/process files change.

if [[ "${GITHUB_EVENT_NAME:-}" != "pull_request" ]]; then
  echo "Skipping evolution docs check outside pull_request events."
  exit 0
fi

base_sha="${BASE_SHA:-}"
head_sha="${HEAD_SHA:-}"

if [[ -z "${base_sha}" || -z "${head_sha}" ]]; then
  echo "Missing BASE_SHA/HEAD_SHA; cannot evaluate changed files."
  exit 1
fi

changed_files="$(git diff --name-only "${base_sha}" "${head_sha}")"

echo "Changed files between ${base_sha}..${head_sha}:"
echo "${changed_files}"

# Any of these paths indicate a code/runtime/build/process change that should
# have corresponding development-history documentation.
if ! grep -Eq '^(apps/simple-term/|crates/simple-term/|Cargo.toml$|Cargo.lock$|\.github/workflows/|\.github/scripts/)' <<< "${changed_files}"; then
  echo "No scoped code/process changes detected; evolution-doc update not required."
  exit 0
fi

# Require at least one evolution doc file update/add/delete.
if grep -Eq '^docs/evolution/.*\.md$' <<< "${changed_files}"; then
  echo "Evolution docs update detected."
  exit 0
fi

echo "ERROR: Code/process changes detected without docs/evolution update."
echo "Please update docs/evolution (and INDEX.md for new entries) to capture rationale and safe-change guidance."
exit 1

