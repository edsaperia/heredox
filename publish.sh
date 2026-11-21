#!/bin/sh
# Publishes web/ to the gh-pages branch. Every script and stylesheet URL gets
# a version stamp so a phone never mixes a new page with cached old scripts.
set -e
cd "$(dirname "$0")"
V=$(cat web/*.js web/index.html web/skins/*/* | sha1sum | cut -c1-10)
rm -rf /tmp/ghp && mkdir /tmp/ghp
cp -r web/index.html web/engine.js web/ai.js web/ui.js web/skins web/.nojekyll /tmp/ghp/
sed -i -E "s#(src|href)=\"([^\"]+\.(js|css))\"#\1=\"\2?v=$V\"#g" /tmp/ghp/index.html
cd /tmp/ghp
git init -q -b gh-pages
git add -A
git -c user.name="Claude" -c user.email="noreply@anthropic.com" commit -qm "Publish $V"
git push -q -f "$(git -C "$OLDPWD" remote get-url origin)" gh-pages
echo "published $V"
