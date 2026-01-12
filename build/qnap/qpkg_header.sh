#!/bin/sh
# QPKG self-extracting package for Unified Hi-Fi Control
# This header is concatenated with payload archives

QPKG_NAME="unified-hifi-control"
QPKG_DISPLAYNAME="Unified Hi-Fi Control"

# Installation directory
QPKG_INSTALL_PATH="/share/CACHEDEV1_DATA/.qpkg"
QPKG_DIR="${QPKG_INSTALL_PATH}/${QPKG_NAME}"

# Find script location and calculate payload offset
# Uses unique marker to avoid false matches
SCRIPT_PATH="$0"
SCRIPT_SIZE=$(sed '/^__PAYLOAD_BEGINS__$/q' "$SCRIPT_PATH" | wc -c)

# Check if running as root
if [ "$(id -u)" != "0" ]; then
    echo "Error: This script must be run as root"
    exit 1
fi

# Create unique temp file and setup cleanup trap
TMP_PAYLOAD=$(mktemp /tmp/qpkg_payload.XXXXXX.tar.gz)
cleanup() {
    rm -f "$TMP_PAYLOAD"
}
trap cleanup EXIT INT TERM

# Extract payload (everything after marker line)
echo "Extracting ${QPKG_DISPLAYNAME}..."
tail -c +$((SCRIPT_SIZE + 1)) "$SCRIPT_PATH" > "$TMP_PAYLOAD"

# Create installation directory
mkdir -p "$QPKG_DIR"

# Extract the archive with error handling
if ! tar -xzf "$TMP_PAYLOAD" -C "$QPKG_DIR"; then
    echo "Error: Failed to extract package"
    exit 1
fi

# Make executables runnable
chmod +x "${QPKG_DIR}/unified-hifi-control" 2>/dev/null || true
for script in "${QPKG_DIR}"/*.sh; do
    [ -f "$script" ] && chmod +x "$script"
done

# Run install script if present
if [ -f "${QPKG_DIR}/install.sh" ]; then
    if ! "${QPKG_DIR}/install.sh"; then
        echo "Warning: install.sh returned non-zero"
    fi
fi

# Register with QNAP
echo "Registering package..."
/sbin/setcfg "${QPKG_NAME}" Name "${QPKG_NAME}" -f /etc/config/qpkg.conf
/sbin/setcfg "${QPKG_NAME}" Display_Name "${QPKG_DISPLAYNAME}" -f /etc/config/qpkg.conf
/sbin/setcfg "${QPKG_NAME}" Version "{{VERSION}}" -f /etc/config/qpkg.conf
/sbin/setcfg "${QPKG_NAME}" Author "Muness Castle" -f /etc/config/qpkg.conf
/sbin/setcfg "${QPKG_NAME}" Install_Path "${QPKG_DIR}" -f /etc/config/qpkg.conf
/sbin/setcfg "${QPKG_NAME}" Enable TRUE -f /etc/config/qpkg.conf
/sbin/setcfg "${QPKG_NAME}" Service_Port 8088 -f /etc/config/qpkg.conf

# Start the service
echo "Starting ${QPKG_DISPLAYNAME}..."
"${QPKG_DIR}/unified-hifi-control.sh" start

echo "Installation complete!"
exit 0
__PAYLOAD_BEGINS__
