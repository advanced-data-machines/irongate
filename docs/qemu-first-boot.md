# QEMU lesson one: a local offline VM

QEMU provides the virtual machine and devices. KVM accelerates its CPU using the Linux host's hardware virtualization support. The workstation check found an AMD Ryzen 9 5950X, working `kvm_amd`/`kvm`, and user access to `/dev/kvm`. Sandboxing hides that device from ordinary agent commands, so host checks and an actual boot must run outside that sandbox. No group changes or kernel module loading were needed.

QEMU tools were not initially installed; interactive sudo authentication was required. Installation is now verified: QEMU 10.2.1, `qemu-img`, `cloud-localds` and user-accessible KVM. For reproducing this setup on another workstation, the package command is:

```sh
sudo apt-get install --no-install-recommends qemu-system-x86 qemu-utils cloud-image-utils
```

These provide the emulator, `qemu-img` disk tool and `cloud-localds` seed tool. No libvirt or host bridge/network service is being set up. Package installation itself is separate from booting a VM.

## Files and first boot

```sh
just vm-fetch
just vm-prepare
just vm-command
cat target/vm/learn/console-login.txt
just vm-boot
```

`vm-fetch` downloads an exact official Debian 13 genericcloud build from the dated URL in `tools/vm/image.toml`, verifies its pinned SHA-512 and makes the cached base read-only. The digest was checked against Debian's dated HTTPS manifest; an independent OpenPGP signature check is not claimed. Downloads/disks are under ignored `target/vm`, not committed to Git.

`vm-prepare` creates a private 32 GiB virtual writable overlay and a read-only NoCloud seed containing guest-only initialization. It chooses a random guest console password, records it in a private file and creates the `lab` user with sudo inside the guest. It does not print passwords or read host SSH keys. Existing prepared disks are never reset implicitly. This workstation's learning disk is now prepared and its first KVM boot/login have been verified; use `vm-boot` to reopen it instead of preparing it again.

The seed is like a small configuration disk, not an installer. Cloud-init inside Debian uses it to set the guest hostname/user without downloading packages or needing a NIC. `-nic none` removes QEMU's default networking; only guest loopback is expected in this lesson. No host routes, firewall, NICs or bridges are changed, and no host directories or block devices are passed through.

The complete command stays readable in [boot.sh](../tools/vm/boot.sh). `vm-command` prints it without starting anything:

| Option | Meaning |
| --- | --- |
| `-name irongate-learn` | Name the VM process |
| `-machine q35,accel=kvm` | Modern x86 virtual machine with hardware acceleration; no silent emulation fallback |
| `-cpu host` | Expose the host CPU model to this local guest |
| `-smp 4` | Four virtual CPUs |
| `-m 8192` | 8192 MiB of guest RAM |
| First `-drive` | Writable qcow2 overlay; unchanged base image is its backing file |
| Second `-drive` | Read-only seed with guest initialization |
| `-nic none` | No VM network interface |
| `-display none` | No graphical window in lesson one |
| `-serial mon:stdio` | Terminal is both guest serial console and QEMU monitor |
| `-no-reboot` | A guest reboot exits QEMU instead of starting it again |

## What to try inside the guest

Wait for cloud-init to finish, then log in as `lab` using the local credential file. These commands run **inside Debian**, not on your workstation:

```sh
cat /etc/os-release
uname -r
lsblk
ip -brief address
ip route
sudo poweroff
```

Expect Debian, a virtual disk grown toward 32 GiB and only loopback networking. Poweroff is a clean shutdown. The 32 GiB is virtual capacity: qcow2 consumes host disk space as blocks are written, subject to available host space.

The verified guest is Debian 13.7 from the pinned image, kernel `6.12.117+deb13-cloud-amd64`, with four CPUs, a 32 GiB disk, only `lo` and no routes. QEMU monitor checks confirmed KVM enabled and no network devices. The first boot waited about two minutes for the deliberately absent NIC; inside the guest we masked `systemd-networkd-wait-online.service` and reset its failure, then shut down cleanly. The original seed also emitted a recoverable temporary password-setup warning, although console login worked. New preparation supplies the password during user creation and masks the offline wait after initialization; a brand-new disk may still encounter the base image's initial network-wait timeout. Re-enable the guest wait service deliberately if a later lesson needs it; no workstation unit was changed.

The second boot finished cloud-init at about 4.14 seconds of guest uptime, with no errors/recoverable errors and no failed services. It still had only loopback and no routes, and was shut down cleanly. Redacted evidence is saved under `target/vm/learn/boot-verification.json`. This proves the offline VM setup, not any gateway/portal behavior.

Press **Ctrl-a, then c** to switch from the guest serial console to QEMU's monitor. Try `info kvm`, `info block` and `info network`, then Ctrl-a, c to return. This monitor controls the virtual machine, not a shell in Debian. **Ctrl-a, then x** exits QEMU immediately; use it only if needed because it is equivalent to removing guest power. Prefer `sudo poweroff` for normal shutdown.

The automated [packet-lab lesson](linux-packet-lab.md) now provisions a separate package image with temporary userspace NAT, then tests with `-nic none` and read-only seed artifacts. It preserves the interactive learning disk and requires no guest login.

Further interactive lessons can inspect the base/overlay relationship and preserve a clean checkpoint. Networking comes after that: temporary QEMU userspace NAT for guest package preparation, then the isolated namespaces/VLAN topology in [the VM demo](vm-demo.md). The original interactive lesson remains separate from that automated packet lab.

References: [QEMU accelerators](https://www.qemu.org/docs/master/system/introduction.html), [QEMU options](https://www.qemu.org/docs/master/system/qemu-manpage.html), and the [official Debian image location](https://cloud.debian.org/images/cloud/trixie/20261116-2664/).
