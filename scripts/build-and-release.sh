#!/usr/bin/env bash

set -e -x

BUILD_TARGET=(aarch64-unknown-linux-musl x86_64-pc-windows-gnu)

temp_dir=$(mktemp -d)
version=$(git describe --tags | tr -d '\n')

for target in ${BUILD_TARGET[@]}; do
  echo "Building for target ${target}"

  cross build --verbose --release --target="${target}"

  # Copy the binary to the temporary directory
  if [[ $target == "x86_64-pc-windows-gnu" ]]; then
    ext=.exe
  else
    ext=
  fi

  rls_path="target/${target}/release"

  if [ -e "${rls_path}/bouffalo-cli${ext}" ]; then
    rls_name="bouffalo-cli-${version}-${target}"
    cp "${rls_path}/bouffalo-cli${ext}" "${temp_dir}/${rls_name}${ext}"
    gzip "${temp_dir}/${rls_name}${ext}"
    ghr "${version}" "${temp_dir}/${rls_name}${ext}.gz"
  fi
done

