#!/usr/bin/env bash
# Download the generated cutaways into media/.
#
# The clips are not committed: they were generated from a sandbox whose egress
# policy blocks the Higgsfield CDN, so the bytes could never be pulled in. Run
# this from a machine with normal network access.
#
# Higgsfield CDN links are long-lived but not guaranteed permanent. If one 404s,
# the job IDs are in README.md — re-fetch from your Higgsfield history.
set -euo pipefail

cd "$(dirname "$0")"
mkdir -p media

BASE="https://d8j0ntlcm91z4.cloudfront.net/user_3GIUur2SS2F0j4FthvKoUQ8VEFn"

fetch() {
  local name=$1 url=$2
  if [ -s "media/$name" ]; then
    echo "have    $name"
    return
  fi
  echo "fetch   $name"
  curl -fSL --retry 3 -o "media/$name.part" "$url"
  mv "media/$name.part" "media/$name"
}

# Skit 1 shot 1 — folder slams onto a cluttered desk
fetch cutaway-folder-slam.mp4 \
  "$BASE/hf_20260801_134450_7fad3aa6-6536-4280-b406-f0bb8c511a7c.mp4"

# Skit 2 shot 11 — terminal scrolling in a dark room
fetch cutaway-terminal-glow.mp4 \
  "$BASE/hf_20260801_135605_d1de71a0-7fa5-443f-b0b9-14886d34e14b.mp4"

# Skit 4 shot 9 — photocopier degrading each successive page
fetch cutaway-photocopier.mp4 \
  "$BASE/hf_20260801_142614_5d759fe1-79e5-4a9c-a263-956aea84787d.mp4"

# Skit 3 shot 1 — corkboard, pinned pages, red string
fetch cutaway-corkboard.mp4 \
  "$BASE/hf_20260801_143625_1e7f477d-c2e2-4d74-91a6-9be66c769b60.mp4"

echo
ls -lh media/
echo
echo "All clips are silent 1080x1920 @ 5s. Before concatenating with live"
echo "footage, give them an audio track — see 'Cutting it together' in README.md."
