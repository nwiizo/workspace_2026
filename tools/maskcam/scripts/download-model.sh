#!/bin/sh
# Download and verify the YuNet model used by maskcam.
# Usage: ./scripts/download-model.sh [destination]
set -eu

readonly model_url='https://github.com/opencv/opencv_zoo/raw/main/models/face_detection_yunet/face_detection_yunet_2026may.onnx'
readonly model_sha256='ebafce4e3c118d6554634be5c27ab333b4c047a9a8c3faf1d7cf93101c22f0f0'
readonly destination="${1:-models/face_detection_yunet_2026may.onnx}"
destination_dir="$(dirname "$destination")"
readonly destination_dir
readonly temporary_file="${destination}.download"

mkdir -p "$destination_dir"
trap 'rm -f "$temporary_file"' EXIT HUP INT TERM

curl --fail --location --silent --show-error "$model_url" --output "$temporary_file"

actual_sha256=$(shasum -a 256 "$temporary_file" | awk '{print $1}')
if [ "$actual_sha256" != "$model_sha256" ]; then
    echo "model checksum mismatch: expected $model_sha256, got $actual_sha256" >&2
    exit 1
fi

mv "$temporary_file" "$destination"
echo "downloaded verified model to $destination"
