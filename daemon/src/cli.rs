use std::io::IsTerminal;
use std::process::ExitCode;

use atriumd::ha::client::Session;
use atriumd::ha::prefs;
use atriumd::ha::registry::AreaPrefs;
use atriumd::ha::url;

fn read_token() -> Option<String> {
    if let Ok(token) = std::env::var("ATRIUM_TOKEN") {
        if !token.trim().is_empty() {
            return Some(token.trim().to_string());
        }
    }
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut buffer = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer).ok()?;
    let token = buffer.trim().to_string();
    (!token.is_empty()).then_some(token)
}

pub async fn call(target: &str, entity: &str, action: &str, data: Option<&str>) -> ExitCode {
    let endpoint = match url::parse(target) {
        Ok(endpoint) => endpoint,
        Err(e) => {
            eprintln!("atriumd: {e}");
            return ExitCode::FAILURE;
        }
    };
    let payload: serde_json::Value = match data {
        Some(raw) => match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(e) => {
                eprintln!("atriumd: action data is not valid JSON: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => serde_json::json!({}),
    };

    let resolved = match atriumd::ha::action::resolve(entity, action, &payload) {
        Ok(resolved) => resolved,
        Err(e) => {
            eprintln!("atriumd: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "resolved to {}.{} on {}",
        resolved.domain, resolved.service, resolved.entity_id
    );

    let Some(token) = read_token() else {
        eprintln!("atriumd: no token (set ATRIUM_TOKEN or pipe it on stdin)");
        return ExitCode::FAILURE;
    };
    let mut session = match Session::connect(&endpoint, &token).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("atriumd: {e}");
            return ExitCode::FAILURE;
        }
    };
    match session
        .call_service(&resolved.domain, &resolved.service, &resolved.entity_id, resolved.data)
        .await
    {
        Ok(()) => {
            println!("accepted by Home Assistant");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("atriumd: {e}");
            ExitCode::FAILURE
        }
    }
}

pub async fn probe(target: &str) -> ExitCode {
    let endpoint = match url::parse(target) {
        Ok(endpoint) => endpoint,
        Err(e) => {
            eprintln!("atriumd: {e}");
            return ExitCode::FAILURE;
        }
    };
    if endpoint.plaintext {
        eprintln!(
            "atriumd: warning — {} is plaintext; the access token will be sent unencrypted",
            endpoint.origin
        );
    }
    let Some(token) = read_token() else {
        eprintln!("atriumd: no token (set ATRIUM_TOKEN or pipe it on stdin)");
        return ExitCode::FAILURE;
    };

    let mut session = match Session::connect(&endpoint, &token).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("atriumd: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("connected to {} (Home Assistant {})", endpoint.origin, session.ha_version);

    let snapshot = match session.snapshot().await {
        Ok(snapshot) => snapshot,
        Err(e) => {
            eprintln!("atriumd: {e}");
            return ExitCode::FAILURE;
        }
    };
    let live: Vec<String> = snapshot
        .states
        .iter()
        .filter_map(|s| s.get("entity_id").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect();
    println!(
        "registry: {} areas, {} devices, {} entities, {} live states",
        snapshot.registry.areas.len(),
        snapshot.registry.devices.len(),
        snapshot.registry.entities.len(),
        live.len(),
    );

    let imported = session
        .lovelace_config()
        .await
        .as_ref()
        .and_then(|config| prefs::from_lovelace_config(config, true));
    let (label, area_prefs) = match imported {
        Some(p) => ("imported from your Home Assistant dashboard", p),
        None => ("Atrium defaults (no generated dashboard to import)", AreaPrefs::default()),
    };
    println!("area preferences: {label}");

    let names = display_names(&snapshot);
    let tabs = snapshot
        .registry
        .project_tabs(&live, &area_prefs, |id| names.get(id).cloned().unwrap_or_else(|| id.to_string()));

    println!("\npanel would show {} tabs, no favorites required:", tabs.len());
    for tab in &tabs {
        println!("  {:<22} {:>3} entities", tab.title, tab.entity_ids.len());
    }
    println!(
        "\ntotal {} entities",
        tabs.iter().map(|t| t.entity_ids.len()).sum::<usize>()
    );

    if std::env::args().any(|a| a == "--rows") {
        use std::collections::HashMap;
        let states: HashMap<&str, &serde_json::Value> = snapshot
            .states
            .iter()
            .filter_map(|s| Some((s.get("entity_id")?.as_str()?, s)))
            .collect();
        for tab in &tabs {
            println!("\n{}", tab.title);
            for id in &tab.entity_ids {
                let Some(entity) = states.get(id.as_str()) else { continue };
                let Some(row) = atriumd::ha::model::row(entity, None, None) else { continue };
                let controls: Vec<String> = row
                    .controls
                    .iter()
                    .map(|c| format!("{c:?}"))
                    .collect();
                println!(
                    "  {:<34} {:<14} {}",
                    row.name.chars().take(34).collect::<String>(),
                    row.display_state.chars().take(14).collect::<String>(),
                    controls.join(",")
                );
            }
        }
    }
    ExitCode::SUCCESS
}

fn display_names(snapshot: &atriumd::ha::client::Snapshot) -> std::collections::HashMap<String, String> {
    snapshot
        .states
        .iter()
        .filter_map(|s| {
            let id = s.get("entity_id")?.as_str()?;
            let name = s
                .get("attributes")
                .and_then(|a| a.get("friendly_name"))
                .and_then(|v| v.as_str())?;
            Some((id.to_string(), name.to_string()))
        })
        .collect()
}
