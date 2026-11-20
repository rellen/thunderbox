//! The part that turns the declarations of a dashboard into a registry.
//!
//! A dashboard names each data source that it needs, and it states the type of
//! that data source in the `kind` field. This module makes one data source for
//! each declaration. Each type of data source reads its own options, so this
//! module passes the declaration on and reads no option itself.
//!
//! A feature selects the types that a build of the program holds.
//! ADR 0011 in `docs/contributor/adr` states that rule.

use std::sync::Arc;

use thunderbox_core::{Dashboard, Error, Result, SourceDecl};
use thunderbox_source::{DataSource, Registry};
use thunderbox_source_sysinfo::SysinfoSource;
use thunderbox_source_testdata::TestData;

#[cfg(feature = "prometheus")]
use thunderbox_source_prometheus::Prometheus;

/// The types of data source that this build of the program can make.
const KINDS: &[&str] = &[
    thunderbox_source_sysinfo::KIND,
    thunderbox_source_testdata::KIND,
    #[cfg(feature = "prometheus")]
    thunderbox_source_prometheus::KIND,
];

/// Returns a registry that holds one data source for each declaration.
///
/// # Errors
///
/// This function returns an error in each of these conditions:
///
/// - the dashboard declares a type of data source that this build of the
///   program does not hold;
/// - a declaration holds an option that its own type of data source rejects.
pub fn build(dashboard: &Dashboard) -> Result<Registry> {
    let mut registry = Registry::new();
    for (uid, declaration) in &dashboard.sources {
        registry.insert(uid.clone(), one(uid, declaration)?);
    }
    Ok(registry)
}

/// Returns one data source.
fn one(uid: &str, declaration: &SourceDecl) -> Result<Arc<dyn DataSource>> {
    match declaration.kind.as_str() {
        thunderbox_source_sysinfo::KIND => {
            Ok(Arc::new(SysinfoSource::from_declaration(declaration)?))
        }
        thunderbox_source_testdata::KIND => Ok(Arc::new(TestData::from_declaration(declaration)?)),
        #[cfg(feature = "prometheus")]
        thunderbox_source_prometheus::KIND => {
            Ok(Arc::new(Prometheus::from_declaration(declaration)?))
        }
        other => Err(Error::Dashboard(format!(
            "the data source {uid:?} has the type {other:?}, and this program \
             makes these types: {}",
            KINDS.join(", ")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dashboard(sources: &str) -> Dashboard {
        let text = format!("title = \"x\"\n{sources}");
        Dashboard::from_toml(&text).expect("The test must hold a correct dashboard.")
    }

    #[test]
    fn build_makes_one_data_source_for_each_declaration() {
        let dash = dashboard(
            "[source.a]\nkind = \"testdata\"\nseries = 2\n\
             [source.b]\nkind = \"testdata\"\n",
        );
        let registry = build(&dash).unwrap();
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.uids().collect::<Vec<_>>(), vec!["a", "b"]);
        assert_eq!(registry.get("a").unwrap().descriptor().kind, "testdata");
    }

    #[test]
    fn build_returns_a_registry_with_no_data_source_for_a_dashboard_that_names_none() {
        assert!(build(&dashboard("")).unwrap().is_empty());
    }

    #[test]
    fn build_returns_an_error_for_a_type_that_this_program_does_not_hold() {
        let error = build(&dashboard("[source.a]\nkind = \"influxdb\"\n"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("influxdb"),
            "The message must name the type. {error}"
        );
        assert!(
            error.contains("testdata"),
            "The message must list the types that the program holds. {error}"
        );
    }

    #[test]
    fn build_returns_an_error_that_names_a_data_source_with_a_wrong_option() {
        let error = build(&dashboard(
            "[source.slow]\nkind = \"testdata\"\nspeed = 2\n",
        ))
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("speed"),
            "The message must name the option. {error}"
        );
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn build_makes_a_prometheus_data_source() {
        let dash = dashboard(
            "[source.prom]\nkind = \"prometheus\"\n\
             url = \"http://127.0.0.1:9090\"\nmin_step = \"15s\"\n",
        );
        let registry = build(&dash).unwrap();
        let descriptor = registry.get("prom").unwrap().descriptor();
        assert_eq!(descriptor.kind, "prometheus");
        assert!(descriptor.downsamples);
    }

    #[test]
    fn build_makes_the_data_source_that_reads_this_machine() {
        let registry = build(&dashboard("[source.machine]\nkind = \"sysinfo\"\n")).unwrap();
        assert_eq!(
            registry.get("machine").unwrap().descriptor().kind,
            "sysinfo"
        );
    }

    #[test]
    fn the_dashboard_inside_the_program_names_a_type_that_this_program_makes() {
        let text = include_str!("default.toml");
        let dash = Dashboard::from_toml(text).expect("The dashboard must be correct.");
        build(&dash).expect("The program must be able to make its own data sources.");
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn the_dashboard_that_reads_prometheus_is_correct() {
        // The project offers this file to a person who runs a Prometheus
        // server. A fault in it would reach that person, and not a test.
        let text = include_str!("../../../dashboards/prometheus.toml");
        let dash = Dashboard::from_toml(text).expect("The dashboard must be correct.");
        let registry = build(&dash).expect("The program must be able to make the data source.");
        assert_eq!(
            registry.get("prom").unwrap().descriptor().kind,
            "prometheus"
        );
        assert_eq!(dash.panels.len(), 4);
        assert_eq!(
            dash.variables[0].name, "instance",
            "The file offers a variable to a person who runs a server."
        );
    }

    /// The project offers this file to every person, because the testdata data
    /// source needs no server. A fault in it would reach that person, and not
    /// a test.
    #[test]
    fn the_dashboard_that_shows_the_variables_is_correct() {
        let text = include_str!("../../../dashboards/variables.toml");
        let dash = Dashboard::from_toml(text).expect("The dashboard must be correct.");
        build(&dash).expect("The program must be able to make the data source.");
        assert_eq!(dash.variables.len(), 1);
        assert_eq!(dash.variables[0].name, "signal");
        for panel in &dash.panels {
            for target in &panel.targets {
                assert_eq!(
                    target.expr, "$signal",
                    "Every panel of this dashboard must read the variable."
                );
            }
        }
    }

    /// The project offers this file to every person, as it offers the file of
    /// the variables. A fault in it would reach that person, and not a test.
    #[test]
    fn the_dashboard_that_shows_the_row_panels_is_correct() {
        let text = include_str!("../../../dashboards/rows.toml");
        let dash = Dashboard::from_toml(text).expect("The dashboard must be correct.");
        build(&dash).expect("The program must be able to make the data source.");
        let rows: Vec<&str> = dash
            .panels
            .iter()
            .filter(|panel| panel.kind == thunderbox_core::PanelKind::Row)
            .map(|panel| panel.title.as_str())
            .collect();
        assert_eq!(rows, vec!["load", "network", "processes"]);
        assert!(
            dash.panels
                .iter()
                .any(|panel| panel.collapsed == Some(true)),
            "The file shows a row panel that starts collapsed."
        );
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn build_returns_an_error_for_a_prometheus_address_that_is_not_correct() {
        let error = build(&dashboard(
            "[source.prom]\nkind = \"prometheus\"\nurl = \"nine thousand\"\n",
        ))
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("nine thousand"),
            "The message is wrong. {error}"
        );
    }
}
