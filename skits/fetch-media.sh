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

fetch cutaway-folder-slam.mp4 \
  "$BASE/hf_20260801_134450_7fad3aa6-6536-4280-b406-f0bb8c511a7c.mp4"
fetch cutaway-terminal-glow.mp4 \
  "$BASE/hf_20260801_135605_d1de71a0-7fa5-443f-b0b9-14886d34e14b.mp4"

echo
ls -lh media/
echo
echo "Both clips are silent 1080x1920. Before concatenating with live footage,"
echo "give them an audio track — see 'Cutting it together' in README.md."
