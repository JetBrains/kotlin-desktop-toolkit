#!/usr/bin/env bash
set -eux -o pipefail

# Requirements: qemu and mkisofs (genisoimage on Ubuntu, cdrtools in Homebrew)

ubuntu_codename="resolute"
local_ssh_port=2223

images_cache_path="${HOME}/.cache/cloud-images"
mkdir -p "${images_cache_path}"

image_filename="${ubuntu_codename}-server-cloudimg-arm64.img"
image_filepath="${images_cache_path}/${image_filename}"
etag_filepath="${image_filepath}-etag.txt"
new_etag_filepath="${image_filepath}-etag.txt.new"

if [ -f "${etag_filepath}" ]; then
  cat "${etag_filepath}"
fi

curl \
  -v \
  --etag-compare "${etag_filepath}" \
  --etag-save "${new_etag_filepath}" \
  "https://cloud-images.ubuntu.com/${ubuntu_codename}/current/${image_filename}" \
  --output "${image_filepath}"

# Workaround for https://github.com/curl/curl/issues/15728
if [ -s "${new_etag_filepath}" ]; then
  mv "${new_etag_filepath}" "${etag_filepath}"
else
  rm "${new_etag_filepath}"
fi

#create the cloud-init NoCloud metadata disk file
tmppath="${images_cache_path}/cloudinitmetadata-${ubuntu_codename}"

rm -Rf "${tmppath}"
mkdir -p "${tmppath}"

private_ssh_key_path="${tmppath}/id_rsa"
ssh-keygen -t ed25519 -f "${private_ssh_key_path}" -P ""
chmod 0600 "${private_ssh_key_path}"

public_key=$(cat "${private_ssh_key_path}.pub")

cat <<EOF >"${tmppath}/meta-data"
instance-id: iid-kdt-test-${ubuntu_codename}
local-hostname: kdt-test-${ubuntu_codename}
EOF

script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
sed -e "s|\${PUBLIC_KEY}|${public_key}|g" "${script_dir}/cloud-init-user-data.yaml" > "${tmppath}/user-data"

#create the cloud-init optical drive
mkisofs -output "${tmppath}/cidata.iso" -volid cidata -joliet -rock "${tmppath}/meta-data" "${tmppath}/user-data"

qemu-img create -f qcow2 -B qcow2 -b "${image_filepath}" "${tmppath}/temp_overlay.qcow2" 20G

echo "Connect via: ssh -i ${private_ssh_key_path} -o \"StrictHostKeyChecking no\" -o \"UserKnownHostsFile=/dev/null\" ubuntu@localhost -p${local_ssh_port}"
# Use -Dorg.gradle.workers.max=1 to avoid OOM, e.g.:
# clear; ./gradlew -Dorg.gradle.workers.max=1 -Ptests.wayland.ime=fcitx5 --console verbose testWayland --rerun --tests org.jetbrains.desktop.linux.tests.WaylandTests

##boot the machine up
qemu-system-aarch64 \
  -machine virt,gic-version=3 \
  -accel hvf \
  -cpu host \
  -smp 3 -m 8096 \
  -vga none -device virtio-gpu,edid=on,xres=3000,yres=1500 \
  -cdrom "${tmppath}/cidata.iso" \
  -drive file=/opt/homebrew/share/qemu/edk2-aarch64-code.fd,if=pflash,format=raw,readonly=on \
  -device virtio-scsi-pci,id=scsi0 -drive file="${tmppath}/temp_overlay.qcow2",if=none,format=qcow2,discard=unmap,id=hda -device scsi-hd,drive=hda,bus=scsi0.0 \
  -device qemu-xhci,id=usb -device usb-kbd,bus=usb.0 -device usb-tablet,bus=usb.0 \
  -netdev id=net0,type=user,hostfwd=tcp::${local_ssh_port}-:22 -device virtio-net-pci,netdev=net0
