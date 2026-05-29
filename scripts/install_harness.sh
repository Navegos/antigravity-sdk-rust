#!/bin/bash
# install_harness.sh - Download and extract the compiled localharness binary
# from the PyPI wheel without requiring Python, pip, or virtual environments.

set -euo pipefail

VERSION="0.1.1"
PLATFORM=""
ARCH="$(uname -m)"
OS="$(uname -s)"

case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64) PLATFORM="macosx_11_0_arm64" ;;
            x86_64)
                echo "============================================================"
                echo "ERROR: Intel (x86_64) macOS is not supported."
                echo "============================================================"
                echo ""
                echo "The upstream google-antigravity package on PyPI does not"
                echo "provide a valid wheel for Intel macOS (x86_64)."
                echo "See: https://pypi.org/project/google-antigravity/"
                echo ""
                echo "Recommended alternatives:"
                echo "  1. Use an Apple Silicon (arm64) Mac"
                echo "  2. Run inside a Linux container or VM (Docker, OrbStack, etc.)"
                echo "  3. Use a Linux cloud instance (x86_64 or aarch64)"
                echo ""
                echo "If upstream releases an Intel macOS build in the future,"
                echo "update this script to re-enable x86_64 support."
                exit 1
                ;;
            *) echo "Unsupported macOS architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            x86_64) PLATFORM="manylinux_2_17_x86_64" ;;
            aarch64) PLATFORM="manylinux_2_17_aarch64" ;;
            *) echo "Unsupported Linux architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported operating system: $OS"
        exit 1
        ;;
esac

echo "Fetching localharness v${VERSION} for ${PLATFORM}..."

# Query the PyPI JSON API to locate the download URL for the wheel matching our platform
JSON_DATA=$(curl -sSf "https://pypi.org/pypi/google-antigravity/${VERSION}/json" || true)
DOWNLOAD_URL=""
if [ -n "$JSON_DATA" ]; then
    DOWNLOAD_URL=$(echo "$JSON_DATA" | grep -o 'https://[^"]*\.whl' | grep "$PLATFORM" | head -n 1 || true)
fi

if [ -z "$DOWNLOAD_URL" ]; then
    # Fallback to standard URL construction if grep parsing fails
    DOWNLOAD_URL="https://files.pythonhosted.org/packages/py3/g/google-antigravity/google_antigravity-${VERSION}-py3-none-${PLATFORM}.whl"
fi

WHEEL_FILE="google_antigravity-${VERSION}-py3-none-${PLATFORM}.whl"

echo "Downloading wheel from: ${DOWNLOAD_URL}"
if ! curl -sSLf -o "${WHEEL_FILE}" "${DOWNLOAD_URL}"; then
    echo "ERROR: Failed to download the localharness wheel for platform '${PLATFORM}'."
    echo "This platform may not be supported or published on PyPI."
    exit 1
fi

# Validate the downloaded wheel is not empty or corrupt
if [[ ! -s "${WHEEL_FILE}" ]]; then
    echo "ERROR: Downloaded wheel is empty (0 bytes)."
    echo "This usually means the platform '${PLATFORM}' is not supported upstream."
    echo "Check: https://pypi.org/project/google-antigravity/#files"
    rm -f "${WHEEL_FILE}"
    exit 1
fi

# Validate it is a valid ZIP (wheel) file
if ! unzip -t "${WHEEL_FILE}" > /dev/null 2>&1; then
    echo "ERROR: Downloaded wheel is not a valid ZIP/wheel file."
    echo "The file may be corrupt or the platform may be unsupported upstream."
    rm -f "${WHEEL_FILE}"
    exit 1
fi

echo "Extracting localharness binary..."
# Extract just the binary from the wheel (which is a standard ZIP file)
if ! unzip -q -o "${WHEEL_FILE}" "google/antigravity/bin/localharness"; then
    echo "ERROR: Could not extract 'google/antigravity/bin/localharness' from wheel."
    echo "The wheel may not contain a binary for this platform."
    rm -f "${WHEEL_FILE}"
    exit 1
fi

# Move the binary to the bin/ directory
mkdir -p bin
mv google/antigravity/bin/localharness bin/localharness
chmod +x bin/localharness

# Validate the extracted binary is not empty
if [[ ! -s bin/localharness ]]; then
    echo "ERROR: Extracted localharness binary is empty (0 bytes)."
    echo "The wheel may not contain a valid binary for this platform."
    rm -f bin/localharness
    rm -rf google "${WHEEL_FILE}"
    exit 1
fi

# Clean up temporary files
rm -rf google "${WHEEL_FILE}"

echo "SUCCESS: localharness installed at ./bin/localharness"
echo "To configure, run: export ANTIGRAVITY_HARNESS_PATH=\$(pwd)/bin/localharness"
