# rkbuild
My small kbuild TUI for automatic tmpfs kernel builds

*rkbuild: rust kbuild*

## What does this project do?
- Downloads the kernel from kernel.org or a git repo (e.g. github)
- Sets a kernel install postfix (e.g. rkbuild => vmlinuz-linux-rkbuild)
- Optionally runs ```make xconfig```
- Builds the kernel inside tmpfs
- Clean old kernel modules
- Installs the kernel and its modules
- Cleans, builds and installs nvidia dkms
- generates initramfs

## Requirements/Assumptions

This project is *currently* not ready for general use, since it requires/assumes a few things:

- Arch
- 32 GiB of RAM with ~16 GiB RAM available
- nvidia-dkms, dkms
- For kernel.org downloads: A 6.x kernel
- root privileges
- /etc/mkinitcpio.d/linux-*postfix*.preset

## I'm feeling lucky, I want to try it.

*Disclaimer: This tool **will** delete old kernel versions. I'm taking no responsibility for any wrong ```sudo rm -rf``` commands done 
by this tool (And any other critical and non critical bugs). You have been warned!*

```cargo build --release && sudo target/release/rkbuild```

The rest should be explained by the TUI
