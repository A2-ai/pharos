#!/usr/bin/env sh
case "$1" in
  *Username*) printf "x-oauth-basic" ;;
  *)          printf "%s" "${GITHUB_TOKEN:-$GH_TOKEN}" ;;
esac
