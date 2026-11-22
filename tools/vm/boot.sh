#!/usr/bin/env bash
# Readable first lesson: one offline VM, existing private overlay, serial console.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
vm_dir="$project_dir/target/vm/learn"

command=(
    qemu-system-x86_64
    -name irongate-learn
    -machine q35,accel=kvm
    -cpu host
    -smp 4
    -m 8192
    -drive "file=$vm_dir/disk.qcow2,format=qcow2,if=virtio"
    -drive "file=$vm_dir/seed.img,format=raw,if=virtio,readonly=on"
    -nic none
    -display none
    -serial mon:stdio
    -no-reboot
)

if [[ "${1:-}" == --print && $# == 1 ]]; then
    printf '%q ' "${command[@]}"
    printf '\n'
    exit 0
fi
if [[ $# != 0 ]]; then
    printf 'Usage: bash tools/vm/boot.sh [--print]\n' >&2
    exit 2
fi
if ! command -v qemu-system-x86_64 >/dev/null; then
    printf 'QEMU is missing. See docs/qemu-first-boot.md for installation.\n' >&2
    exit 1
fi
if [[ ! -r /dev/kvm || ! -w /dev/kvm ]]; then
    printf 'KVM is unavailable to this process; check outside sandboxing. No fallback was selected.\n' >&2
    exit 1
fi
for file in disk.qcow2 seed.img; do
    if [[ ! -f "$vm_dir/$file" ]]; then
        printf 'Missing %s; run just vm-prepare first.\n' "$file" >&2
        exit 1
    fi
done
exec "${command[@]}"
