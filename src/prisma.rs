use json_value_merge::Merge;
use std::{env, fs};
use zed_extension_api::{
    self as zed,
    serde_json::{self, Value},
    settings::LspSettings,
    Extension, Result, Worktree,
};

const PACKAGE_NAME: &str = "@prisma/language-server";
const LANGUAGE_SERVER_ID: &str = "prisma-language-server";

const PIN_PRISMA_KEY: &str = "pinToPrisma6";
const PIN_PRISMA_VERSION_KEY: &str = "pinnedPrismaVersion";
const DEFAULT_PINNED_PRISMA_VERSION: &str = "6.0.13";

struct PrismaExtension {
    did_find_server: bool,
    using_pinned_version: bool,
}

impl PrismaExtension {
    fn server_exists(&self, server_path: &str) -> bool {
        fs::metadata(server_path).is_ok_and(|stat| stat.is_file())
    }

    fn server_script_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        let workspace_settings = self
            .language_server_workspace_configuration(language_server_id, worktree)
            .ok()
            .flatten();

        let prisma_settings = workspace_settings
            .as_ref()
            .and_then(|settings| settings.get("prisma"))
            .and_then(Value::as_object);

        let pinned_version_override = prisma_settings
            .and_then(|settings| settings.get(PIN_PRISMA_VERSION_KEY))
            .and_then(Value::as_str)
            .map(|version| version.trim())
            .filter(|version| !version.is_empty())
            .map(String::from);

        let should_install_pinned_version_configured = prisma_settings
            .and_then(|settings| settings.get(PIN_PRISMA_KEY))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let should_install_pinned_version =
            should_install_pinned_version_configured || pinned_version_override.is_some();

        let version_changed = self.using_pinned_version != should_install_pinned_version;
        self.did_find_server &= version_changed;

        let target_version = if let Some(version) = pinned_version_override {
            version
        } else if should_install_pinned_version {
            DEFAULT_PINNED_PRISMA_VERSION.to_string()
        } else {
            zed::npm_package_latest_version(PACKAGE_NAME)?
        };

        let (os, _arch) = zed::current_platform();
        let server_path = format!(
            "node_modules/{server_script}",
            server_script = match os {
                zed::Os::Mac | zed::Os::Linux => ".bin/prisma-language-server",
                zed::Os::Windows => "@prisma/language-server/dist/bin.js",
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

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?
                .is_none_or(|installed_version| installed_version != target_version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &target_version);
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
        self.using_pinned_version = should_install_pinned_version;
        Ok(server_path)
    }
}

impl zed::Extension for PrismaExtension {
    fn new() -> Self {
        Self {
            did_find_server: false,
            using_pinned_version: false,
        }
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
            .map(|settings| settings.initialization_options.clone())
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree).map(|lsp_settings| {
            let default_settings = {
                serde_json::json!({
                    "prisma": {"enableDiagnostics": true}
                })
            };

            Some(
                lsp_settings
                    .settings
                    .clone()
                    .map(|mut settings| {
                        settings.merge(&default_settings);
                        settings
                    })
                    .unwrap_or(default_settings),
            )
        })
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.server_script_path(language_server_id, worktree)?;
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
