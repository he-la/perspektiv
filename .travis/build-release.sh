#!/bin/bash
# Build all combinations of arch and features
#
# A hug part of this script has been lifted from
# https://raw.githubusercontent.com/carllerche/travis-rust-matrix/746e0f0e175903d57fa066c61a31c3d62dfa2f95/test

set -e
set -x

# Abort the script with an error message
abort() {
  echo $1 >&2
  exit 1
}

# Extract some info from rustc
rustc_info() {
  rustc -Vv | grep $1 | awk '{print $2}'
}

is_lib() {
  grep -q '\[lib\]' Cargo.toml || [ -e src/lib.rs ]
}

# Check that the rustc command exists
command -v rustc > /dev/null || abort "rustc cannot be found on path"

# Ensure that $ARCH has been specified
[ -z $ARCH ] && abort 'no $ARCH specified'

SOURCE_HOST=$(rustc_info "host")
SOURCE_ARCH=$(echo $SOURCE_HOST | cut -d - -f 1)
SOURCE_OS=$(echo $SOURCE_HOST | cut -d - -f 2-)
TARGET_HOST=$ARCH-$SOURCE_OS
RUSTC_ROOT=$(rustc --print sysroot)
RUSTC_LIB="$RUSTC_ROOT/lib/rustlib"
RUSTC_DATE=$(rustc_info "commit-date")

# Setup rust

[ $ARCH != $SOURCE_ARCH ] && {
  # Ensure supported arch
  [[ "i686 x86_64" =~ $ARCH ]] || abort "unsuported ARCH $ARCH"

  # Make sure that the target arch's lib is present
  [ -d "$RUSTC_LIB/$TARGET_HOST" ] || {
    RELEASE=$(rustc_info "release")

    if echo $RELEASE | grep -q beta; then
      VERSION="beta"
    elif echo $RELEASE | grep -q nightly; then
      VERSION="nightly"
    else
      VERSION=$RELEASE
    fi

    DIST_URL=https://static.rust-lang.org/dist/rust-$VERSION-$TARGET_HOST.tar.gz
    RUST_STD=rust-std-$TARGET_HOST
    LIB_SUBDIR=lib/rustlib/$TARGET_HOST

    echo "Installing alternate rust for arch=$ARCH"

    # Download
    curl $DIST_URL | tar -zxf -
    pushd rust-$VERSION-$TARGET_HOST

    # If the new stdlib directory exists, copy files out of there, otherwise
    # fallback to the old location
    if [ -d "$RUST_STD" ]; then
      # rust-std-i686-unknown-linux-gnu/lib/rustlib/i686-unknown-linux-gnu/lib
      mv $RUST_STD/$LIB_SUBDIR $RUSTC_LIB/
    else
      mv rustc/$LIB_SUBDIR $RUSTC_LIB/
    fi

    popd
  }
}

# Finally, Build

mapfile -t FEATURES < <( .travis/get-features Cargo.toml )
for flist in "${FEATURES[@]}"; do
    echo "Building with features $flist"
    eval "cargo build --release --target $TARGET_HOST --verbose --features $flist" || exit 1
    echo "Copying binary to release folder"
    rdir=$(eval "echo release/$ARCH/$(echo $flist | tr ' ' +)")
    mkdir -p "$rdir"
    cp "target/$TARGET_HOST/release/perspektiv" "$rdir"
    echo "Done"
done
