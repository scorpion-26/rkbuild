use std::{
    env,
    fs::{self},
    os::{self, unix::prelude::PermissionsExt},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{anyhow, Result};

use crate::{
    choices::{EnumInput, TextInput, TextInputType},
    download::download,
    process::{run_process, run_process_user},
    ui::UI,
};

enum Source {
    Git,
    KernelOrg,
}

struct BuildContext<'a> {
    ui: Arc<Mutex<UI<'a>>>,
    source: Source,

    linux_tar_xz: String,
    source_dir: String,
    config: String,
    postfix: String,
    xconfig: bool,
}

impl<'a> BuildContext<'a> {
    // Downloads source and prepares for build. Changes directory to the source dir
    pub fn prepare_source(&mut self) -> Result<()> {
        self.ui
            .lock()
            .unwrap()
            .log()
            .append(String::from("Preparing source..."));

        // Postfix (To differentiate kernel versions)
        let mut postfix_input =
            TextInput::new(TextInputType::String, "Please enter install postfix: ");
        let postfix = postfix_input.output();
        let future = self.ui().input().set(postfix_input.choice());
        future.wait();
        self.postfix = postfix.lock().unwrap().clone();

        // Where the config is
        let mut config_input =
            TextInput::new(TextInputType::String, "Please enter .config location: ");
        let config = config_input.output();
        let future = self.ui().input().set(config_input.choice());
        future.wait();

        let abs_config = fs::canonicalize(config.lock().unwrap().clone())?;
        self.config = String::from(abs_config.to_str().unwrap());
        self.ui().log().append(self.config.clone());

        let mut xconfig_input = EnumInput::new(vec!["Yes".into(), "No".into()], "Open xconfig?");
        let xconfig_idx = xconfig_input.output();
        let future = self.ui().input().set(xconfig_input.choice());
        future.wait();
        let idx = xconfig_idx.lock().unwrap();
        self.xconfig = match *idx {
            0 => true,
            _ => false,
        };

        // Where the sources are
        let mut location_input = EnumInput::new(
            vec![String::from("kernel.org"), String::from("git")],
            "Please select source location: ",
        );
        let location_idx = location_input.output();
        let future = self.ui().input().set(location_input.choice());
        future.wait();
        let idx = location_idx.lock().unwrap();
        match *idx {
            0 => {
                self.source = Source::KernelOrg;
                self.download_kernel_org()?;
                self.extract()?;
                self.verify()?;
                self.untar()?;
            }
            1 => {
                self.source = Source::Git;
                self.download_git()?;
            }
            _ => return Err(anyhow!("Index out of bounds!")),
        };

        env::set_current_dir(self.source_dir.clone())?;

        // Clean
        self.ui().log().append(String::from("Cleaning..."));
        run_process("make mrproper", &mut self.ui)?;

        // Copy config
        fs::copy(&self.config, format!("{}/.config", self.source_dir))?;

        // Run xconfig
        if self.xconfig {
            // Add root as xhost
            run_process_user("xhost +si:localuser:root", &mut self.ui)?;
            run_process(
                format!(
                    "make KERNELRELEASE=\"$(make -s kernelversion)-{postfix}\" xconfig",
                    postfix = self.postfix
                )
                .as_str(),
                &mut self.ui,
            )?;
        }

        Ok(())
    }

    pub fn build(&mut self) -> Result<()> {
        self.ui().log().append(String::from("Compiling kernel"));
        let cmd = format!("make KERNELRELEASE=\"$(make -s kernelversion)-{postfix}\" -j$(nproc) && make KERNELRELEASE=\"$(make -s kernelversion)-{postfix}\" modules -j$(nproc)", postfix = self.postfix);
        run_process(cmd.as_str(), &mut self.ui)?;
        Ok(())
    }

    pub fn install(&mut self) -> Result<()> {
        // Remove old modules, to avoid keeping stale mods into all eternity
        self.ui()
            .log()
            .append("Removing old Kernel modules from /usr/lib/modules".into());
        // Delete everything that ends in "-{postfix}", so we clean old versions.
        // Don't care if this fails, since rm fails when directory doesn't exist.
        let _ = run_process(format!("rm -rf *-{}", self.postfix).as_str(), &mut self.ui);

        // Install modules
        self.ui().log().append(String::from(
            "Installing Kernel modules to /usr/lib/modules",
        ));
        let cmd = format!("ZSTD_CLEVEL=19 make KERNELRELEASE=\"$(make -s kernelversion)-{}\" INSTALL_MOD_STRIP=1 modules_install -j$(nproc)", self.postfix);
        run_process(cmd.as_str(), &mut self.ui)?;

        // Install vmlinuz
        self.ui()
            .log()
            .append(String::from("Installing Kernel to /boot"));
        run_process(
            format!(
                "cp $(make -s image_name) /boot/vmlinuz-linux-{}",
                self.postfix
            )
            .as_str(),
            &mut self.ui,
        )?;
        // Needed for systemd
        // From arch PKGBUILD: "systemd expects to find the kernel here to allow hibernation"
        self.ui()
            .log()
            .append(String::from("Installing Kernel to /usr/lib/modules"));
        run_process(
            format!(
                "cp $(make -s image_name) /usr/lib/modules/$(make -s kernelversion)-{}/vmlinuz",
                self.postfix
            )
            .as_str(),
            &mut self.ui,
        )?;

        // I don't care about dynamic DKMS support, if I need to recompile DKMS I just can
        // recompile the kernel. This allows for the exclusion of ~60MiB of headers in the final
        // install
        self.nvidia_dkms()?;

        // Unlink
        self.ui()
            .log()
            .append(String::from("Removing symlinks in /usr/lib/modules"));
        run_process(
            format!(
                "rm /usr/lib/modules/$(make -s kernelversion)-{}/{{source,build}}",
                self.postfix
            )
            .as_str(),
            &mut self.ui,
        )?;

        self.mkinitcpio()?;

        Ok(())
    }

    fn download_kernel_org(&mut self) -> Result<()> {
        // Ask for version
        let mut version_input =
            TextInput::new(TextInputType::Version, "Please select kernel version: ");
        let version = version_input.output();
        let future = self.ui().input().set(version_input.choice());
        future.wait();

        // Check if exists
        self.linux_tar_xz = format!("/tmp/linux/linux-{}.tar.xz", version.lock().unwrap());
        let mut linux_tar = PathBuf::from(&self.linux_tar_xz);
        linux_tar.set_extension("");
        if Path::new(&self.linux_tar_xz).exists() || linux_tar.exists() {
            self.ui().log().append(format!(
                "{} already exists, skipping download",
                self.linux_tar_xz
            ));
            // Already downloaded
            return Ok(());
        }

        download(
            format!(
                "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-{}.tar.xz",
                version.lock().unwrap()
            ),
            format!("/tmp/linux/linux-{}.tar.xz", version.lock().unwrap()),
            self.ui.clone(),
        )?;

        Ok(())
    }

    fn download_git(&mut self) -> Result<()> {
        let mut repo_input = TextInput::new(TextInputType::String, "Please enter git repository: ");
        let repo = repo_input.output();
        let future = self.ui().input().set(repo_input.choice());
        future.wait();
        self.ui().log().append(repo.lock().unwrap().clone());

        fs::create_dir_all("/tmp/linux")?;
        self.source_dir = String::from("/tmp/linux/linux");

        // Check if exists
        if Path::new("/tmp/linux/linux").is_dir() {
            self.ui()
                .log()
                .append("/tmp/linux/linux already exists, skipping download".into());
            // Already downloaded
            return Ok(());
        }

        run_process(
            format!(
                "git clone --depth=1 {} {}",
                repo.lock().unwrap(),
                self.source_dir
            )
            .as_str(),
            &mut self.ui,
        )?;
        Ok(())
    }

    fn extract(&mut self) -> Result<()> {
        let mut buf = PathBuf::from(self.linux_tar_xz.as_str());
        buf.set_extension("");
        if buf.exists() {
            self.ui().log().append(format!(
                "{} already exists, skipping extract!",
                buf.into_os_string().into_string().unwrap()
            ));
            return Ok(());
        }

        self.ui()
            .log()
            .append(format!("Extracting {}", self.linux_tar_xz));
        run_process(format!("unxz {}", self.linux_tar_xz).as_str(), &mut self.ui)
    }
    fn verify(&mut self) -> Result<()> {
        self.ui().log().append(String::from("Verify: TODO!"));
        Ok(())
    }

    fn untar(&mut self) -> Result<()> {
        let mut dir = PathBuf::from(self.linux_tar_xz.as_str());
        // linux.tar.xz -> linux.tar
        dir.set_extension("");
        // linux.tar -> linux
        dir.set_extension("");

        self.source_dir = dir.clone().into_os_string().into_string().unwrap();
        let tar = format!("{}.tar", self.source_dir);

        if dir.exists() {
            self.ui().log().append(format!(
                "{} already exists, skipping untar!",
                self.source_dir
            ));
            return Ok(());
        }
        self.ui().log().append(format!("Untaring {}", tar));
        run_process(
            format!("tar -xvf {} -C /tmp/linux", tar).as_str(),
            &mut self.ui,
        )
    }

    fn ui(&self) -> MutexGuard<'_, UI<'a>> {
        self.ui.lock().unwrap()
    }

    fn nvidia_dkms(&mut self) -> Result<()> {
        self.ui().log().append("Building nvidia dkms module".into());
        // Output: nvidia-dkms xxx.xx-x
        // We only care about xxx.xx
        let gather_nvidia_ver = "pacman -Q nvidia-dkms | grep -oP \"[0-9]*\\.[0-9]*\\.?[0-9]*\"";

        // Remove old version
        run_process(
            format!(
                "dkms remove nvidia/$({}) -k $(make -s kernelversion)-{}",
                gather_nvidia_ver, self.postfix
            )
            .as_str(),
            &mut self.ui,
        )?;

        // Build
        run_process(
            format!(
                "dkms install nvidia/$({}) -k $(make -s kernelversion)-{}",
                gather_nvidia_ver, self.postfix
            )
            .as_str(),
            &mut self.ui,
        )?;

        Ok(())
    }

    // Assumes, that preset linux-{postfix} is available
    fn mkinitcpio(&mut self) -> Result<()> {
        self.ui().log().append("Generating initramfs".into());
        if run_process(
            format!("mkinitcpio -p linux-{}", self.postfix).as_str(),
            &mut self.ui,
        )
        .is_err()
        {
            self.ui().log().append(format!(
                "Failed to generate mkinitcpio! Does /etc/mkinitcpio.d/linux-{}.preset exist?",
                self.postfix
            ));
        }
        Ok(())
    }

    pub fn clean(&mut self) -> Result<()> {
        self.ui().log().append("Cleaning tmpfs".into());
        run_process("rm -rf /tmp/linux", &mut self.ui)?;
        Ok(())
    }
}

pub fn build_thread<'a>(ui: Arc<Mutex<UI<'a>>>) {
    let mut ctx: BuildContext<'a> = BuildContext {
        ui,
        source: Source::Git,
        linux_tar_xz: String::new(),
        source_dir: String::new(),
        config: String::new(),
        postfix: String::new(),
        xconfig: false,
    };

    // Check for root. We need root for installing
    unsafe {
        if libc::getuid() != 0 {
            ctx.ui()
                .log()
                .append(String::from("rkbuild needs root privileges!"));
            return;
        }
    }

    loop {
        ctx.ui()
            .log()
            .append(String::from("rkbuild - Linux kernel build TUI"));
        if let Some(err) = ctx.prepare_source().err() {
            ctx.ui()
                .log()
                .append(format!("Error preparing source: {}", err));
            continue;
        }
        if let Some(err) = ctx.build().err() {
            ctx.ui()
                .log()
                .append(format!("Error building kernel: {}", err));
            continue;
        }
        if let Some(err) = ctx.install().err() {
            ctx.ui()
                .log()
                .append(format!("Error installing kernel: {}", err));
            continue;
        }
        if let Some(err) = ctx.clean().err() {
            ctx.ui()
                .log()
                .append(format!("Error cleaning tmpfs: {}", err));
            continue;
        }
        ctx.ui().log().append(String::from("Done!"));
    }
}
