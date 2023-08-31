use anyhow::{anyhow, Result};

use std::{
    env,
    fs::{self, File, Permissions},
    io::Write,
    os::unix::prelude::PermissionsExt,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::ui::UI;

type Curl = curl::easy::Easy;

pub fn download(url: String, output: String, ui: Arc<Mutex<UI>>) -> Result<()> {
    ui.lock()
        .unwrap()
        .log()
        .append(format!("Starting to download {}.", url));

    let mut curl = Curl::new();
    curl.url(url.clone().as_str())?;
    curl.progress(true)?;
    let mut data = Vec::new();

    {
        let mut transfer = curl.transfer();
        transfer.write_function(|out_data| {
            data.extend_from_slice(out_data);
            Ok(out_data.len())
        })?;

        let progress_ui = ui.clone();
        let mut last_progress_update = Instant::now();
        transfer.progress_function(move |total, downloaded, _, _| {
            if total == 0.0 {
                // Don't care
                return true;
            }

            // Check if sufficient time has passed
            if last_progress_update.elapsed().as_millis() < 1000 {
                // Not enough delta time
                return true;
            }

            last_progress_update = Instant::now();

            let percent = (downloaded as f32 / total as f32) * 100.0;
            progress_ui.lock().unwrap().log().append(format!(
                "Downloaded {:.2} MiB/{:.2} MiB, {:.2}%",
                downloaded / (1024.0 * 1024.0),
                total / (1024.0 * 1024.0),
                percent
            ));
            true
        })?;
        transfer.perform()?;
    }

    if curl.response_code()? != 200 {
        return Err(anyhow!("Invalid response: {}", curl.response_code()?));
    }

    // Write
    let path = Path::new(&output);
    fs::create_dir_all(path.parent().unwrap())?;
    let mut out = File::create(path)?;
    out.write_all(&data)?;

    ui.lock()
        .unwrap()
        .log()
        .append(format!("Finished downloading {}.", url));

    Ok(())
}
