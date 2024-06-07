#!/bin/bash
set -eo pipefail
IFS=$'\n\t'

script_dir=$(dirname "$(realpath "$0")")
cd "${script_dir}/.."

name=$1
version=$2

if [[ -z "${name}" ]] || [[ -z "${version}" ]]; then
  echo "ERROR: name/version missing"
  exit 1
fi

versioned="r2:/cdn-soupbawx-com/${name}/docker-reroll-${version}-x86_64-unknown-linux-musl"
latest=r2:/cdn-soupbawx-com/${name}/docker-reroll-latest-x86_64-unknown-linux-musl

if rclone lsf "${versioned}" | grep -q .; then
  echo "ERROR: ${versioned} already exists"
  exit 1
fi

set -x

rclone copyto "target/x86_64-unknown-linux-musl/release/${name}" "${versioned}"
rclone copyto "target/x86_64-unknown-linux-musl/release/${name}" "${latest}"
