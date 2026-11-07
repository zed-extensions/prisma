use json_value_merge::Merge;
use std::{env, fs};
use zed_extension_api::{
    self as zed,
    serde_json::{self, Value},
    settings::LspSettings,
    Extension, Result, Worktree,
};

const LEGACY_PACKAGE_NAME: &str = "@prisma/language-server";
const LANGUAGE_SERVER_ID: &str = "prisma-language-server";

const SERVER_MODE_KEY: &str = "serverMode";
const PIN_PRISMA_KEY: &str = "pinToPrisma6";
const PINNED_PRISMA_VERSION: &str = "6.19.0-hotfix.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerMode {
    Auto,
    Prisma8,
    Legacy,
}

impl ServerMode {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "prisma8" | "prisma-8" | "next" | "v8" | "8" => Self::Prisma8,
            "legacy" | "prisma7" | "prisma6" | "v7" | "v6" | "7" | "6" => Self::Legacy,
            _ => Self::Auto,
        }
    }
}

struct PrismaExtension {
    did_find_legacy_server: bool,
    using_pinned_version: bool,
}

impl PrismaExtension {
    fn server_exists(&self, server_path: &str) -> bool {
        fs::metadata(server_path).is_ok_and(|stat| stat.is_file())
    }

    /// Finds candidate subpackage directories in a monorepo (e.g. `packages/database`, `apps/web`).
    fn find_candidate_packages(worktree: &Worktree) -> Vec<String> {
        let mut candidates = Vec::new();
        let root = worktree.root_path();

        // 1. Inspect root package.json for "workspaces"
        if let Ok(pkg_json_str) = worktree.read_text_file("package.json") {
            if let Ok(pkg_json) = serde_json::from_str::<Value>(&pkg_json_str) {
                let mut add_workspace = |val: &Value| {
                    if let Some(s) = val.as_str() {
                        let trimmed = s.trim().trim_end_matches("/*").trim_end_matches('/');
                        if !trimmed.is_empty() && !candidates.contains(&trimmed.to_string()) {
                            let check_dirs = [trimmed.to_string(), format!("{root}/{trimmed}")];
                            for dir in &check_dirs {
                                if let Ok(entries) = fs::read_dir(dir) {
                                    for entry in entries.flatten() {
                                        if let Ok(file_type) = entry.file_type() {
                                            if file_type.is_dir() {
                                                if let Some(sub_name) = entry.file_name().to_str() {
                                                    let rel = format!("{trimmed}/{sub_name}");
                                                    if !candidates.contains(&rel) {
                                                        candidates.push(rel);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            candidates.push(trimmed.to_string());
                        }
                    }
                };

                if let Some(workspaces) = pkg_json.get("workspaces") {
                    if let Some(arr) = workspaces.as_array() {
                        for item in arr {
                            add_workspace(item);
                        }
                    } else if let Some(packages) =
                        workspaces.get("packages").and_then(Value::as_array)
                    {
                        for item in packages {
                            add_workspace(item);
                        }
                    }
                }
            }
        }

        // 2. Also check common monorepo directories: packages, apps, modules, services, libs
        for base in ["packages", "apps", "modules", "services", "libs"] {
            let check_dirs = [base.to_string(), format!("{root}/{base}")];
            for dir in &check_dirs {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                if let Some(sub_name) = entry.file_name().to_str() {
                                    let rel = format!("{base}/{sub_name}");
                                    if !candidates.contains(&rel) {
                                        candidates.push(rel);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback well-known subpackage paths if read_dir is restricted in sandbox
        for known in [
            "packages/database",
            "packages/db",
            "packages/schema",
            "packages/backend",
            "packages/server",
            "packages/api",
            "packages/core",
            "apps/api",
            "apps/server",
            "apps/backend",
            "apps/web",
        ] {
            if !candidates.iter().any(|c| c == known) {
                candidates.push(known.to_string());
            }
        }

        candidates
    }

    /// Determines whether the project uses Prisma 8 or Legacy based on settings,
    /// config files, package.json dependencies, and schema directives.
    fn detect_server_mode(&self, worktree: &Worktree) -> ServerMode {
        // 1. Check explicit user configuration in settings.json
        if let Ok(settings) = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree) {
            if let Some(prisma_settings) = settings.settings.as_ref().and_then(|s| s.get("prisma"))
            {
                if let Some(mode_str) = prisma_settings.get(SERVER_MODE_KEY).and_then(Value::as_str)
                {
                    let mode = ServerMode::from_str(mode_str);
                    if mode != ServerMode::Auto {
                        return mode;
                    }
                }
                if prisma_settings
                    .get(PIN_PRISMA_KEY)
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    return ServerMode::Legacy;
                }
            }
        }

        // 2. Check for Prisma 8 configuration files at worktree root
        for config_path in [
            "prisma.config.ts",
            "prisma.config.js",
            "prisma.config.mjs",
            "prisma.config.cjs",
            "prisma/prisma.config.ts",
            "prisma/prisma.config.js",
        ] {
            if worktree.read_text_file(config_path).is_ok() {
                return ServerMode::Prisma8;
            }
        }

        // 3. Check for Prisma 8 contract files at root
        for contract_path in [
            "src/prisma/contract.prisma",
            "prisma/contract.prisma",
            "contract.prisma",
            "src/prisma/contract.ts",
            "prisma/contract.ts",
        ] {
            if worktree.read_text_file(contract_path).is_ok() {
                return ServerMode::Prisma8;
            }
        }

        // 4. Check schema files for Prisma 8 directive at root
        for schema_path in [
            "schema.prisma",
            "prisma/schema.prisma",
            "src/prisma/schema.prisma",
        ] {
            if let Ok(content) = worktree.read_text_file(schema_path) {
                if Self::has_prisma8_directive(&content) {
                    return ServerMode::Prisma8;
                }
            }
        }

        // 5. Check root package.json dependencies
        if let Ok(pkg_json) = worktree.read_text_file("package.json") {
            if Self::has_prisma8_in_package_json(&pkg_json) {
                return ServerMode::Prisma8;
            }
        }

        // 6. Check candidate subpackages in monorepos
        let candidates = Self::find_candidate_packages(worktree);
        for pkg_rel in &candidates {
            for config_name in [
                "prisma.config.ts",
                "prisma.config.js",
                "prisma.config.mjs",
                "prisma.config.cjs",
                "prisma/prisma.config.ts",
            ] {
                if worktree
                    .read_text_file(&format!("{pkg_rel}/{config_name}"))
                    .is_ok()
                {
                    return ServerMode::Prisma8;
                }
            }

            for contract_name in [
                "src/prisma/contract.prisma",
                "prisma/contract.prisma",
                "contract.prisma",
                "src/prisma/contract.ts",
            ] {
                if worktree
                    .read_text_file(&format!("{pkg_rel}/{contract_name}"))
                    .is_ok()
                {
                    return ServerMode::Prisma8;
                }
            }

            if let Ok(sub_pkg_json) = worktree.read_text_file(&format!("{pkg_rel}/package.json")) {
                if Self::has_prisma8_in_package_json(&sub_pkg_json) {
                    return ServerMode::Prisma8;
                }
            }

            for schema_rel in [
                "schema.prisma",
                "prisma/schema.prisma",
                "src/prisma/schema.prisma",
            ] {
                if let Ok(content) = worktree.read_text_file(&format!("{pkg_rel}/{schema_rel}")) {
                    if Self::has_prisma8_directive(&content) {
                        return ServerMode::Prisma8;
                    }
                }
            }
        }

        ServerMode::Legacy
    }

    fn has_prisma8_directive(content: &str) -> bool {
        for line in content.lines().take(20) {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                let comment = trimmed.trim_start_matches('/').trim();
                if comment.starts_with("use prisma-8") || comment.starts_with("use prisma-next") {
                    return true;
                }
            }
        }
        false
    }

    fn has_prisma8_in_package_json(content: &str) -> bool {
        if let Ok(json) = serde_json::from_str::<Value>(content) {
            let check_section = |section: Option<&Value>| -> bool {
                if let Some(deps) = section.and_then(Value::as_object) {
                    for (pkg, ver) in deps {
                        if pkg.starts_with("@prisma/orm-")
                            || pkg == "@prisma/cli-engine"
                            || pkg == "@prisma/client-runtime"
                        {
                            return true;
                        }
                        if pkg == "prisma" || pkg == "@prisma/client" {
                            if let Some(ver_str) = ver.as_str() {
                                let clean = ver_str.trim_start_matches(['^', '~', '>', '=']);
                                if clean.starts_with("8.") {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            };

            if check_section(json.get("dependencies"))
                || check_section(json.get("devDependencies"))
                || check_section(json.get("peerDependencies"))
            {
                return true;
            }
        }
        false
    }

    /// Spawns the Prisma ORM 8 language server via `prisma lsp --stdio`.
    fn prisma8_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let node = zed::node_binary_path()?;
        let root = worktree.root_path();

        let mut env = worktree.shell_env();
        // Prevent ANSI color output from corrupting the JSON-RPC stdio stream
        env.push(("FORCE_COLOR".to_string(), "0".to_string()));
        env.push(("NO_COLOR".to_string(), "1".to_string()));
        env.push(("NODE_ENV".to_string(), "production".to_string()));
        env.push(("ELECTRON_RUN_AS_NODE".to_string(), "1".to_string()));
        env.push(("ELECTRON_NO_ASAR".to_string(), "1".to_string()));

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        // 1. Check local node_modules/prisma/dist/prisma.js at root
        let root_prisma_dist = format!("{root}/node_modules/prisma/dist/prisma.js");
        if worktree
            .read_text_file("node_modules/prisma/package.json")
            .is_ok()
            || self.server_exists(&root_prisma_dist)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );
            return Ok(zed::Command {
                command: node,
                args: vec![root_prisma_dist, "lsp".to_string(), "--stdio".to_string()],
                env,
            });
        }

        // 2. Check local node_modules/.bin/prisma at root
        let root_prisma_bin = format!("{root}/node_modules/.bin/prisma");
        if self.server_exists(&root_prisma_bin) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );
            return Ok(zed::Command {
                command: root_prisma_bin,
                args: vec!["lsp".to_string(), "--stdio".to_string()],
                env,
            });
        }

        // 3. Check candidate subpackages in monorepos (e.g. packages/database/node_modules/...)
        let candidates = Self::find_candidate_packages(worktree);
        for pkg_rel in &candidates {
            let rel_prisma_pkg = format!("{pkg_rel}/node_modules/prisma/package.json");
            let rel_prisma_dist = format!("{pkg_rel}/node_modules/prisma/dist/prisma.js");
            let abs_prisma_dist = format!("{root}/{pkg_rel}/node_modules/prisma/dist/prisma.js");

            if worktree.read_text_file(&rel_prisma_pkg).is_ok()
                || self.server_exists(&rel_prisma_dist)
                || self.server_exists(&abs_prisma_dist)
            {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                return Ok(zed::Command {
                    command: node,
                    args: vec![abs_prisma_dist, "lsp".to_string(), "--stdio".to_string()],
                    env,
                });
            }

            let rel_prisma_bin = format!("{pkg_rel}/node_modules/.bin/prisma");
            let abs_prisma_bin = format!("{root}/{pkg_rel}/node_modules/.bin/prisma");
            if self.server_exists(&rel_prisma_bin) || self.server_exists(&abs_prisma_bin) {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                return Ok(zed::Command {
                    command: abs_prisma_bin,
                    args: vec!["lsp".to_string(), "--stdio".to_string()],
                    env,
                });
            }
        }

        // 4. Check hoisted stores at root (Bun: .bun/prisma@8... or pnpm: .pnpm/prisma@8...)
        let bun_store = format!("{root}/node_modules/.bun");
        if let Ok(entries) = fs::read_dir(&bun_store) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("prisma@8.") {
                        let dist_path =
                            format!("{bun_store}/{name}/node_modules/prisma/dist/prisma.js");
                        if self.server_exists(&dist_path) {
                            zed::set_language_server_installation_status(
                                language_server_id,
                                &zed::LanguageServerInstallationStatus::None,
                            );
                            return Ok(zed::Command {
                                command: node,
                                args: vec![dist_path, "lsp".to_string(), "--stdio".to_string()],
                                env,
                            });
                        }
                    }
                }
            }
        }

        // 5. Check if prisma binary is in PATH
        if let Some(prisma_bin) = worktree.which("prisma") {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );
            return Ok(zed::Command {
                command: prisma_bin,
                args: vec!["lsp".to_string(), "--stdio".to_string()],
                env,
            });
        }

        // 6. Check if bun is available in a Bun-managed project
        if worktree.read_text_file("bun.lock").is_ok()
            || worktree.read_text_file("bun.lockb").is_ok()
        {
            if let Some(bun_bin) = worktree.which("bun") {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                return Ok(zed::Command {
                    command: bun_bin,
                    args: vec![
                        "x".to_string(),
                        "prisma".to_string(),
                        "lsp".to_string(),
                        "--stdio".to_string(),
                    ],
                    env,
                });
            }
        }

        // 7. Fallback: check if npx is available to run prisma@latest lsp
        if let Some(npx_bin) = worktree.which("npx") {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );
            return Ok(zed::Command {
                command: npx_bin,
                args: vec![
                    "-y".to_string(),
                    "prisma@latest".to_string(),
                    "lsp".to_string(),
                    "--stdio".to_string(),
                ],
                env,
            });
        }

        let err_msg = "Prisma ORM 8 was detected, but the 'prisma' CLI could not be found.\n\
            Please run `npm install prisma@latest` in your project or install it globally with `npm install -g prisma`."
            .to_string();

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Failed(err_msg.clone()),
        );

        Err(err_msg)
    }

    /// Spawns the legacy `@prisma/language-server` for Prisma <= 7.
    fn legacy_server_script_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        let should_install_pinned_version = self
            .language_server_workspace_configuration(language_server_id, worktree)
            .ok()
            .flatten()
            .is_some_and(|settings| {
                settings
                    .get("prisma")
                    .and_then(|prisma_settings| {
                        prisma_settings.get(PIN_PRISMA_KEY).and_then(Value::as_bool)
                    })
                    .unwrap_or(false)
            });

        let version_changed = self.using_pinned_version != should_install_pinned_version;
        self.did_find_legacy_server &= !version_changed;

        let target_version = if should_install_pinned_version {
            PINNED_PRISMA_VERSION.to_string()
        } else {
            zed::npm_package_latest_version(LEGACY_PACKAGE_NAME)?
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
        if self.did_find_legacy_server && server_exists {
            return Ok(server_path);
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        if !server_exists
            || zed::npm_package_installed_version(LEGACY_PACKAGE_NAME)?
                .is_none_or(|installed_version| installed_version != target_version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(LEGACY_PACKAGE_NAME, &target_version);
            match result {
                Ok(()) => {
                    if !self.server_exists(&server_path) {
                        return Err(format!(
                            "installed package '{LEGACY_PACKAGE_NAME}' did not contain expected path '{server_path}'",
                        ));
                    }
                }
                Err(error) => {
                    if !self.server_exists(&server_path) {
                        return Err(error);
                    }
                }
            }
        }

        self.did_find_legacy_server = true;
        self.using_pinned_version = should_install_pinned_version;
        Ok(server_path)
    }

    fn legacy_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let server_path = self.legacy_server_script_path(language_server_id, worktree)?;
        let mut env = worktree.shell_env();
        env.push(("FORCE_COLOR".to_string(), "0".to_string()));
        env.push(("NO_COLOR".to_string(), "1".to_string()));

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
            env,
        })
    }
}

impl zed::Extension for PrismaExtension {
    fn new() -> Self {
        Self {
            did_find_legacy_server: false,
            using_pinned_version: false,
        }
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let mode = self.detect_server_mode(worktree);
        let mut options = match mode {
            ServerMode::Prisma8 => serde_json::json!({
                "completion": {
                    "supportsTriggerParameterHintsCommand": true
                }
            }),
            _ => serde_json::json!({}),
        };

        if let Ok(settings) = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree) {
            if let Some(user_options) = settings.initialization_options {
                options.merge(&user_options);
            }
        }

        Ok(Some(options))
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let default_settings = serde_json::json!({
            "prisma": {
                "enableDiagnostics": true
            }
        });

        if let Ok(lsp_settings) = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree) {
            let mut settings = lsp_settings
                .settings
                .unwrap_or_else(|| serde_json::json!({}));
            settings.merge(&default_settings);
            Ok(Some(settings))
        } else {
            Ok(Some(default_settings))
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // If user defined a custom binary in settings, respect it
        if let Ok(settings) = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree) {
            if let Some(binary) = settings.binary {
                if let Some(path) = binary.path {
                    let args = binary
                        .arguments
                        .unwrap_or_else(|| vec!["--stdio".to_string()]);
                    let mut env = binary
                        .env
                        .map(|e| e.into_iter().collect())
                        .unwrap_or_else(|| worktree.shell_env());
                    env.push(("FORCE_COLOR".to_string(), "0".to_string()));
                    env.push(("NO_COLOR".to_string(), "1".to_string()));
                    return Ok(zed::Command {
                        command: path,
                        args,
                        env,
                    });
                }
            }
        }

        let mode = self.detect_server_mode(worktree);
        match mode {
            ServerMode::Prisma8 => self.prisma8_command(language_server_id, worktree),
            ServerMode::Legacy | ServerMode::Auto => {
                self.legacy_command(language_server_id, worktree)
            }
        }
    }
}

zed::register_extension!(PrismaExtension);
