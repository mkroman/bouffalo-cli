#!/usr/bin/env bash

# This scripts attempts to build the project using `cross` for a given target

if [ $# -ne 1 ]; then
  echo "Usage ${0} <target>"
  exit 1
fi

target=${1}
temp_dir="release-dist"
version=$(git describe --tags | tr -d '\n')

mkdir -p "${temp_dir}"

set -e -x

echo "Building for target ${target}"

cross build --verbose --release --target="${target}"

# Copy the binary to the temporary directory
if [[ $target =~ "-pc-windows-gnu" ]]; then
  ext=.exe
else
  ext=
fi

rls_path="target/${target}/release"
# 
if [ -e "${rls_path}/bouffalo-cli${ext}" ]; then
  rls_name="bouffalo-cli-${version}-${target}"
  cp "${rls_path}/bouffalo-cli${ext}" "${temp_dir}/${rls_name}${ext}"
  gzip "${temp_dir}/${rls_name}${ext}"
fi
