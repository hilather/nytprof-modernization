//! Engine / backend selection for `nytprof-cli`.
//!
//! Spec: `docs/schemas/engine-selection-mvp-v0.md`
//!
//! - CLI: `--engine=<name>` (overrides env)
//! - Env: `NYTPROF_ENGINE=<name>`
//! - Default: `native`
//! - `auto` maps to native until a Perl facade exists

/// Selected report/decode backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// Rust decode/model/report path (default).
    Native,
    /// Pinned oracle under `baseline/6.15` — not wired into this CLI yet.
    Legacy,
}

/// Allowed engine names for error messages and docs.
pub const ALLOWED_ENGINES: &str = "native, legacy, auto";

/// Resolve engine from optional CLI flag value and optional env value.
///
/// Precedence: `cli` overrides `env`; both unset → [`Engine::Native`].
/// `auto` maps to [`Engine::Native`] (documented until Perl facade exists).
///
/// Returns `Err` with a user-facing message when the name is invalid.
pub fn resolve_engine(cli: Option<&str>, env: Option<&str>) -> Result<Engine, String> {
    let raw = cli.or(env).unwrap_or("native");
    parse_engine_name(raw)
}

fn parse_engine_name(name: &str) -> Result<Engine, String> {
    match name.trim().to_ascii_lowercase().as_str() {
        "native" | "auto" => Ok(Engine::Native),
        "legacy" => Ok(Engine::Legacy),
        other => Err(format!(
            "invalid engine '{other}' (allowed: {ALLOWED_ENGINES})"
        )),
    }
}

/// Message printed when `--engine=legacy` / `NYTPROF_ENGINE=legacy` is selected.
pub fn legacy_not_wired_message() -> String {
    "engine=legacy is not wired into nytprof-cli yet.\n\
     \n\
     Legacy reporting uses the pinned oracle install under baseline/6.15\n\
     (e.g. baseline/6.15/install/bin/nytprofhtml). The full Perl facade\n\
     is not yet available from this binary.\n\
     \n\
     Use --engine=native (default) for the Rust report/verify path, or\n\
     invoke the oracle tools directly from baseline/6.15."
        .to_owned()
}

/// Peel a leading `--engine=…` / `--engine …` from argv-style args.
///
/// Returns `(flag_value, remaining_args)`. On parse errors (missing value,
/// duplicate flag), returns `Err` with a user-facing message.
pub fn peel_engine_flag(args: &[String]) -> Result<(Option<String>, Vec<String>), String> {
    let mut engine: Option<String> = None;
    let mut rest = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(val) = a.strip_prefix("--engine=") {
            if engine.is_some() {
                return Err("duplicate --engine flag".into());
            }
            if val.is_empty() {
                return Err(format!(
                    "--engine requires a value (allowed: {ALLOWED_ENGINES})"
                ));
            }
            engine = Some(val.to_string());
            i += 1;
            continue;
        }
        if a == "--engine" {
            if engine.is_some() {
                return Err("duplicate --engine flag".into());
            }
            i += 1;
            let val = args
                .get(i)
                .ok_or_else(|| format!("--engine requires a value (allowed: {ALLOWED_ENGINES})"))?;
            if val.starts_with('-') {
                return Err(format!(
                    "--engine requires a value (allowed: {ALLOWED_ENGINES})"
                ));
            }
            engine = Some(val.clone());
            i += 1;
            continue;
        }
        // Stop peeling at first non-global token; remaining includes subcommand.
        rest.extend_from_slice(&args[i..]);
        break;
    }
    Ok((engine, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_native() {
        assert_eq!(resolve_engine(None, None).unwrap(), Engine::Native);
    }

    #[test]
    fn flag_overrides_env() {
        assert_eq!(
            resolve_engine(Some("native"), Some("legacy")).unwrap(),
            Engine::Native
        );
        assert_eq!(
            resolve_engine(Some("legacy"), Some("native")).unwrap(),
            Engine::Legacy
        );
    }

    #[test]
    fn env_used_when_flag_omitted() {
        assert_eq!(
            resolve_engine(None, Some("legacy")).unwrap(),
            Engine::Legacy
        );
        assert_eq!(
            resolve_engine(None, Some("native")).unwrap(),
            Engine::Native
        );
    }

    #[test]
    fn auto_maps_to_native() {
        assert_eq!(resolve_engine(Some("auto"), None).unwrap(), Engine::Native);
        assert_eq!(resolve_engine(None, Some("auto")).unwrap(), Engine::Native);
        assert_eq!(resolve_engine(Some("AUTO"), None).unwrap(), Engine::Native);
    }

    #[test]
    fn invalid_fails() {
        let err = resolve_engine(Some("bogus"), None).unwrap_err();
        assert!(err.contains("invalid engine"), "{err}");
        assert!(err.contains("native"), "{err}");
        assert!(err.contains("legacy"), "{err}");
        assert!(err.contains("auto"), "{err}");
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(
            resolve_engine(Some("Native"), None).unwrap(),
            Engine::Native
        );
        assert_eq!(
            resolve_engine(Some("LEGACY"), None).unwrap(),
            Engine::Legacy
        );
    }

    #[test]
    fn peel_engine_equals_form() {
        let args = vec!["--engine=native".into(), "report".into(), "foo.out".into()];
        let (eng, rest) = peel_engine_flag(&args).unwrap();
        assert_eq!(eng.as_deref(), Some("native"));
        assert_eq!(rest, vec!["report", "foo.out"]);
    }

    #[test]
    fn peel_engine_space_form() {
        let args = vec![
            "--engine".into(),
            "legacy".into(),
            "verify".into(),
            "foo.out".into(),
        ];
        let (eng, rest) = peel_engine_flag(&args).unwrap();
        assert_eq!(eng.as_deref(), Some("legacy"));
        assert_eq!(rest, vec!["verify", "foo.out"]);
    }

    #[test]
    fn peel_no_engine_flag() {
        let args = vec!["report".into(), "foo.out".into()];
        let (eng, rest) = peel_engine_flag(&args).unwrap();
        assert!(eng.is_none());
        assert_eq!(rest, vec!["report", "foo.out"]);
    }

    #[test]
    fn peel_missing_value_errors() {
        let args = vec!["--engine".into()];
        assert!(peel_engine_flag(&args).is_err());
        let args = vec!["--engine=".into(), "report".into()];
        assert!(peel_engine_flag(&args).is_err());
    }
}
