#!/usr/bin/env bash
set -euo pipefail

branch="gh-pages"
current_branch="$(git rev-parse --abbrev-ref HEAD)"
stashed=false

timestamp="$(date -u "+%Y-%m-%dT%H:%M:%SZ")"

if git show-ref --verify --quiet "refs/heads/${branch}"; then
  echo "${branch} ブランチは既に存在します。再実行モードで続行します。"
  if [ "${current_branch}" != "${branch}" ]; then
    if [ -n "$(git status --porcelain)" ]; then
      git stash push -u -m "bootstrap gh-pages"
      stashed=true
    fi
    git checkout "${branch}"
  fi
else
  if [ -n "$(git status --porcelain)" ]; then
    git stash push -u -m "bootstrap gh-pages"
    stashed=true
  fi
  git checkout --orphan "${branch}"
  git rm -rf . >/dev/null 2>&1 || true
fi

mkdir -p state

if [ ! -f state/articles.json ]; then
  echo "{\"items\":[]}" > state/articles.json
fi

echo "${timestamp}" > last_build.txt

if [ ! -f index.html ]; then
  echo "<!doctype html><html lang=\"ja\"><head><meta charset=\"utf-8\"><title>DEV feed</title></head><body><p><a href=\"feed.xml\">feed.xml</a></p></body></html>" > index.html
fi

: > .nojekyll

if [ ! -f feed.xml ]; then
  echo "<?xml version=\"1.0\" encoding=\"UTF-8\"?>" > feed.xml
  echo "<feed xmlns=\"http://www.w3.org/2005/Atom\"></feed>" >> feed.xml
fi

git add -A

if git diff --cached --quiet; then
  echo "変更がないためコミットをスキップします。"
else
  git commit -m "chore: bootstrap gh-pages"
fi

git push -u origin "${branch}"

if [ "${current_branch}" != "${branch}" ] && [ "${current_branch}" != "HEAD" ]; then
  git checkout "${current_branch}"
fi

if [ "${stashed}" = true ]; then
  git stash pop
fi

echo "${branch} ブランチの準備が完了しました。"
