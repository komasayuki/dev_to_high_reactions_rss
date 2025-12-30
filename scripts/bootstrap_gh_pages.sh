#!/usr/bin/env bash
set -euo pipefail

branch="gh-pages"
current_branch="$(git rev-parse --abbrev-ref HEAD)"

if git show-ref --verify --quiet "refs/heads/${branch}"; then
  echo "${branch} ブランチは既に存在します。"
  exit 0
fi

timestamp="$(date -u "+%Y-%m-%dT%H:%M:%SZ")"

git checkout --orphan "${branch}"

git rm -rf . >/dev/null 2>&1 || true

mkdir -p state

echo "{\"items\":[]}" > state/articles.json

echo "${timestamp}" > last_build.txt

echo "<!doctype html><html lang=\"ja\"><head><meta charset=\"utf-8\"><title>DEV feed</title></head><body><p><a href=\"feed.xml\">feed.xml</a></p></body></html>" > index.html

: > .nojekyll

if [ ! -f feed.xml ]; then
  echo "<?xml version=\"1.0\" encoding=\"UTF-8\"?>" > feed.xml
  echo "<feed xmlns=\"http://www.w3.org/2005/Atom\"></feed>" >> feed.xml
fi

git add -A

git commit -m "chore: bootstrap gh-pages"

git push -u origin "${branch}"

if [ "${current_branch}" != "${branch}" ]; then
  git checkout "${current_branch}"
fi

echo "${branch} ブランチを作成しました。"
