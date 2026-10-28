#!/usr/bin/env bash
# Decides which CI jobs a run needs, so a push only pays for the parts it touched.
# main, tags and manual runs check everything. Branch pushes check what changed since the last push.
# Pull requests from this repo already ran on push, so only forks are checked.
set -euo pipefail

out=${GITHUB_OUTPUT:?}
zero=0000000000000000000000000000000000000000

all() {
	{
		echo "rust=true"
		echo "site=true"
		echo "docs=true"
	} >> "$out"
	echo "Checking everything: $1"
	exit 0
}

none() {
	{
		echo "rust=false"
		echo "site=false"
		echo "docs=false"
	} >> "$out"
	echo "Checking nothing: $1"
	exit 0
}

case "$EVENT" in
	workflow_dispatch) all "manual run" ;;
	pull_request)
		[ "$FORK" = "true" ] || none "a branch of this repo, already checked on push"
		from=$BASE
		;;
	push)
		[ "$REF" = refs/heads/main ] && all "main"
		case "$REF" in refs/tags/*) all "tag" ;; esac
		[ -n "$BEFORE" ] && [ "$BEFORE" != "$zero" ] || all "a new branch"
		from=$BEFORE
		;;
	*) all "event $EVENT" ;;
esac

if ! git fetch --quiet --depth=1 origin "$from"; then
	all "the earlier commit $from can't be fetched"
fi
if ! files=$(git diff --name-only "$from" "$GITHUB_SHA"); then
	all "the diff against $from failed"
fi
echo "$files"

has() { grep -Eq "$1" <<< "$files"; }

has '^\.github/' && all "the workflows changed"

rust=false
site=false
docs=false
# the Linux job checks site/public/data/wheel.json against the code
has '^(crates/|engines/|packaging/|\.config/|Cargo\.(toml|lock)$|rustfmt\.toml$|site/public/data/wheel\.json$)' && rust=true
# the site serves the app's tool windows from crates/app/web
has '^(site/|crates/app/web/|wrangler\.jsonc$)' && site=true
has '(\.md$|^\.markdownlint)' && docs=true
{
	echo "rust=$rust"
	echo "site=$site"
	echo "docs=$docs"
} >> "$out"
echo "rust=$rust site=$site docs=$docs"
