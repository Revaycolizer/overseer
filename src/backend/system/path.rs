pub async fn open_local_path(path: &str) -> anyhow::Result<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Path is empty");
    }

    #[cfg(windows)]
    {
        let script = format!(
            "$target = @'\n{}\n'@; Start-Process -FilePath $target",
            trimmed
        );

        let mut command = std::process::Command::new("powershell");
        command.args(["-NoProfile", "-Command", &script]);

        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);

        let status = command.status()?;
        if !status.success() {
            anyhow::bail!("Failed to open path: {}", trimmed);
        }
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let status = std::process::Command::new("xdg-open")
            .arg(trimmed)
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to open path: {}", trimmed);
        }
        Ok(())
    }
}
