use std::{env, fs};
use zed_extension_api::{self as zed, serde_json, Result};

const PACKAGE_NAME: &str = "@prisma/language-server";

struct PrismaExtension {
    did_find_server: bool,
}

impl PrismaExtension {
    fn server_exists(&self, server_path: &str) -> bool {
        fs::metadata(server_path).map_or(false, |stat| stat.is_file())
    }

    fn server_script_path(&mut self, language_server_id: &zed::LanguageServerId) -> Result<String> {
        let (os, _arch) = zed::current_platform();
        let server_path = format!(
            "node_modules/{server_script}",
            server_script = match os {
                zed::Os::Mac | zed::Os::Linux => ".bin/prisma-language-server",
                zed::Os::Windows => "@prisma/language-server/dist/bin.js"
            }
        );

        let server_exists = self.server_exists(&server_path);
        if self.did_find_server && server_exists {
            return Ok(server_path);
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            match result {
                Ok(()) => {
                    if !self.server_exists(&server_path) {
                        Err(format!(
                            "installed package '{PACKAGE_NAME}' did not contain expected path '{server_path}'",
                        ))?;
                    }
                }
                Err(error) => {
                    if !self.server_exists(&server_path) {
                        Err(error)?;
                    }
                }
            }
        }

        self.did_find_server = true;
        Ok(server_path)
    }
}

impl zed::Extension for PrismaExtension {
    fn new() -> Self {
        Self {
            did_find_server: false,
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        // Todo: support merging user/workspace settings
        // let settings = LspSettings::for_worktree("prisma-language-server", _worktree)
        //     .ok()
        //     .and_then(|lsp_settings| lsp_settings.settings.clone())
        //     .unwrap_or_default();

        Ok(Some(serde_json::json!({
            "prisma": {"enableDiagnostics": true}
        })))
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.server_script_path(language_server_id)?;
        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![
                env::current_dir()
                    .unwrap()
                    .join(&server_path)
                    .to_string_lossy()
                    .to_string(),
                "--stdio".to_string(),
            ],
            env: Default::default(),
        })
    }
}

zed::register_extension!(PrismaExtension);