#!/bin/bash
cargo build "$@"

BINARY_NAME=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[0].targets[] | select(.kind[] | contains("bin")) | .name')
TARGET_DIR=$(cargo metadata --format-version 1 | jq -r '.target_directory')

PROFILE="debug"
[[ "$*" == *"--release"* ]] && PROFILE="release"

if [[ "$*" == *"--target "* ]]; then
    TARGET=$(echo "$*" | grep -oP '(?<=--target )\S+')
    BINARY_PATH="$TARGET_DIR/$TARGET/$PROFILE/$BINARY_NAME"
else
    BINARY_PATH="$TARGET_DIR/$PROFILE/$BINARY_NAME"
fi

sudo chown root:root "$BINARY_PATH"
sudo chmod 4755 "$BINARY_PATH"