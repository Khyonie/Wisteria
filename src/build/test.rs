use std::{collections::HashMap, path::PathBuf, process::Command};

use regex::Regex;

use crate::{
    build::{
        compile,
        resolve::{ClasspathView, ResolvedDependencies, resolve_dependencies_for_views},
        sources,
        task::{TaskOutput, TaskRunner},
    },
    model::{Configuration, Project, ProjectInfo, TestRunner},
    util::{consts, exit_code},
    workspace::paths::resolve_filepath,
};

#[derive(Clone)]
pub struct ImplicitTestTask {
    order: Vec<String>,
}

impl ImplicitTestTask {
    pub fn new() -> Self {
        Self {
            order: vec![
                String::from("resolve"),
                String::from("collect"),
                String::from("compile"),
                String::from("collect-tests"),
                String::from("compile-tests"),
                String::from("test"),
            ],
        }
    }
}

impl Default for ImplicitTestTask {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskRunner for ImplicitTestTask {
    fn invoke(
        &self,
        _info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
        output: &mut TaskOutput<'_>,
    ) -> Result<(), String> {
        let Some(test) = configuration.test() else {
            let error = format!(
                "Configuration has no test settings.\nFix: add `[configuration.{}.test]` with `sources`, `runner`, and `launcher`, or run `wisteria info` to see available tasks.",
                configuration.name()
            );
            output.step_failed("Checking", "test configuration", 1, &error);
            return Err(error);
        };

        let mut regexes: HashMap<&str, Regex> = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());

        output.step_started("Resolving", "dependencies", 1);
        let dependencies = match resolve_dependencies_for_views(
            project,
            configuration,
            &regexes,
            &[
                ClasspathView::MainCompile,
                ClasspathView::TestCompile,
                ClasspathView::TestRuntime,
            ],
        ) {
            Ok(dependencies) => {
                output.step_completed("Resolving", "dependencies", 1, "Done");
                dependencies
            }
            Err(error) => {
                output.step_failed("Resolving", "dependencies", 1, &error);
                return Err(error);
            }
        };

        output.step_started("Collecting", "sources", 2);
        let main_sources = match sources::collect_sources(configuration) {
            Ok(copied_files) => {
                output.step_completed(
                    "Collecting",
                    "sources",
                    2,
                    &source_message(copied_files.len(), "source"),
                );
                copied_files
            }
            Err(error) => {
                output.step_failed("Collecting", "sources", 2, &error);
                return Err(error);
            }
        };

        output.step_started("Compiling", "classes", 3);
        if let Err(error) = compile::compile_sources(
            configuration,
            main_sources,
            dependencies
                .classpath_for(ClasspathView::MainCompile)
                .as_deref(),
            output.renderer(),
        ) {
            output.step_failed("Compiling", "classes", 3, &error);
            return Err(error);
        }
        output.step_completed("Compiling", "classes", 3, "Done");

        output.step_started("Collecting", "test sources", 4);
        let test_sources =
            match sources::collect_source_folders(test.sources(), consts::TEST_SOURCE_OUT_PATH) {
                Ok(copied_files) => {
                    output.step_completed(
                        "Collecting",
                        "test sources",
                        4,
                        &source_message(copied_files.len(), "test source"),
                    );
                    copied_files
                }
                Err(error) => {
                    output.step_failed("Collecting", "test sources", 4, &error);
                    return Err(error);
                }
            };

        output.step_started("Compiling", "test classes", 5);
        let test_compile_classpath = test_compile_classpath(&dependencies);
        if let Err(error) = compile::compile_sources_to(
            configuration,
            test_sources,
            Some(&test_compile_classpath),
            consts::TEST_SOURCE_OUT_PATH,
            consts::TEST_BINARY_OUT_PATH,
            output.renderer(),
        ) {
            output.step_failed("Compiling", "test classes", 5, &error);
            return Err(error);
        }
        output.step_completed("Compiling", "test classes", 5, "Done");

        output.step_started("Running", "tests", 6);
        let result = match test.runner() {
            TestRunner::Junit6 => run_junit6_tests(configuration, &dependencies, &regexes, output),
        };

        match result {
            Ok(()) => {
                output.step_completed("Running", "tests", 6, "Passed");
                Ok(())
            }
            Err(error) => {
                output.step_failed("Running", "tests", 6, &error);
                Err(error)
            }
        }
    }

    fn phase_order(&self) -> &[String] {
        self.order.as_ref()
    }
}

fn run_junit6_tests(
    configuration: &Configuration,
    dependencies: &ResolvedDependencies,
    regexes: &HashMap<&str, Regex>,
    output: &mut TaskOutput<'_>,
) -> Result<(), String> {
    let test = configuration.test().unwrap();
    let launcher = launcher_path(dependencies, test.launcher())?;
    let reports_dir = resolve_filepath(test.reports_dir(), configuration.environment(), regexes)?;
    let runtime_classpath = test_runtime_classpath(dependencies);
    let args = junit6_command_args(
        &launcher.to_string_lossy(),
        &runtime_classpath,
        &reports_dir,
    );

    let mut java_command = Command::new("java");
    java_command.args(&args);

    match java_command.output() {
        Ok(out) => {
            log_junit_process_output(output.renderer(), &out.stdout, &out.stderr);

            if !out.status.success() {
                exit_code::record_external_process_exit_code(out.status);
                return Err(format!("JUnit 6 tests failed with status {}", out.status));
            }
        }
        Err(error) => return Err(format!("Failed to run JUnit 6 launcher: {error}")),
    }

    Ok(())
}

fn launcher_path(dependencies: &ResolvedDependencies, launcher: &str) -> Result<PathBuf, String> {
    let paths = dependencies
        .paths_for_dependency(launcher)
        .unwrap_or_default();

    match paths {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "Test launcher dependency `{launcher}` did not resolve to an artifact.\nFix: make sure `{launcher}` is a single JUnit 6 console launcher dependency, such as `org.junit.platform:junit-platform-console-standalone`, and run `wisteria update {launcher}` if it is not cached."
        )),
        paths => Err(format!(
            "Test launcher dependency `{launcher}` resolved to {} artifacts, but JUnit 6 needs one executable launcher jar.\nFix: point `launcher` at a single archive or Maven dependency, not a folder dependency.",
            paths.len()
        )),
    }
}

fn test_compile_classpath(dependencies: &ResolvedDependencies) -> String {
    classpath_with_entries(
        [String::from(consts::BINARY_OUT_PATH)]
            .into_iter()
            .chain(path_strings(
                dependencies.paths_for(ClasspathView::TestCompile),
            )),
    )
}

fn test_runtime_classpath(dependencies: &ResolvedDependencies) -> String {
    classpath_with_entries(
        [
            String::from(consts::TEST_BINARY_OUT_PATH),
            String::from(consts::BINARY_OUT_PATH),
        ]
        .into_iter()
        .chain(path_strings(
            dependencies.paths_for(ClasspathView::TestRuntime),
        )),
    )
}

fn path_strings(paths: &[PathBuf]) -> impl Iterator<Item = String> + '_ {
    paths.iter().map(|path| path.to_string_lossy().to_string())
}

fn classpath_with_entries(entries: impl IntoIterator<Item = String>) -> String {
    entries
        .into_iter()
        .collect::<Vec<_>>()
        .join(&consts::java_seperator().to_string())
}

fn junit6_command_args(launcher: &str, classpath: &str, reports_dir: &str) -> Vec<String> {
    vec![
        String::from("-jar"),
        launcher.to_string(),
        String::from("execute"),
        String::from("--disable-banner"),
        String::from("--disable-ansi-colors"),
        String::from("--class-path"),
        classpath.to_string(),
        String::from("--scan-classpath"),
        String::from(consts::TEST_BINARY_OUT_PATH),
        String::from("--reports-dir"),
        reports_dir.to_string(),
        String::from("--fail-if-no-tests"),
    ]
}

fn source_message(count: usize, label: &str) -> String {
    format!(
        "{count} {label} {}",
        match count {
            1 => "file",
            _ => "files",
        }
    )
}

fn log_junit_process_output(
    renderer: &mut dyn crate::output::OutputRenderer,
    stdout: &[u8],
    stderr: &[u8],
) {
    let stdout = String::from_utf8_lossy(stdout);
    let stdout = format_junit_console_output(&stdout);
    log_junit_text(renderer, &stdout);

    let stderr = String::from_utf8_lossy(stderr);
    log_junit_text(renderer, &stderr);
}

fn log_junit_text(renderer: &mut dyn crate::output::OutputRenderer, text: &str) {
    let text = text.trim_end_matches(['\r', '\n']);
    if !text.is_empty() {
        renderer.log(text);
    }
}

fn format_junit_console_output(output: &str) -> String {
    let mut details = Vec::new();
    let mut summary = JunitConsoleSummary::default();

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Test run finished after ") {
            summary.finished = Some(trimmed.to_string());
            continue;
        }

        if let Some((group, metric, count)) = parse_junit_summary_line(trimmed) {
            summary.record(group, metric, count);
            continue;
        }

        details.push(line.to_string());
    }

    let mut lines = trim_empty_edge_lines(details);
    if let Some(finished) = summary.finished {
        lines.push(finished);
    }
    if let Some(containers) = summary.containers.format("Containers") {
        lines.push(containers);
    }
    if let Some(tests) = summary.tests.format("Tests") {
        lines.push(tests);
    }

    if lines.is_empty() {
        output.trim_end_matches(['\r', '\n']).to_string()
    } else {
        lines.join("\n")
    }
}

fn parse_junit_summary_line(line: &str) -> Option<(JunitSummaryGroup, String, String)> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?.trim();
    let mut parts = inner.split_whitespace();
    let count = parts.next()?.to_string();
    let group = JunitSummaryGroup::from(parts.next()?)?;
    let metric = parts.collect::<Vec<_>>().join(" ");

    if metric.is_empty() {
        return None;
    }

    Some((group, metric, count))
}

fn trim_empty_edge_lines(lines: Vec<String>) -> Vec<String> {
    let Some(first) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return Vec::new();
    };
    let last = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .unwrap();

    lines[first..=last].to_vec()
}

#[derive(Default)]
struct JunitConsoleSummary {
    finished: Option<String>,
    containers: JunitSummaryCounts,
    tests: JunitSummaryCounts,
}

impl JunitConsoleSummary {
    fn record(&mut self, group: JunitSummaryGroup, metric: String, count: String) {
        match group {
            JunitSummaryGroup::Containers => self.containers.record(metric, count),
            JunitSummaryGroup::Tests => self.tests.record(metric, count),
        }
    }
}

#[derive(Default)]
struct JunitSummaryCounts {
    entries: Vec<(String, String)>,
}

impl JunitSummaryCounts {
    fn record(&mut self, metric: String, count: String) {
        self.entries.push((metric, count));
    }

    fn format(&self, label: &str) -> Option<String> {
        const ORDER: [&str; 6] = [
            "found",
            "skipped",
            "started",
            "aborted",
            "successful",
            "failed",
        ];

        let entries = ORDER
            .into_iter()
            .filter_map(|metric| {
                self.entries
                    .iter()
                    .find(|(entry_metric, _)| entry_metric == metric)
                    .map(|(_, count)| format!("{count} {metric}"))
            })
            .collect::<Vec<_>>();

        if entries.is_empty() {
            return None;
        }

        Some(format!("{label}: {}", entries.join(" | ")))
    }
}

#[derive(Clone, Copy)]
enum JunitSummaryGroup {
    Containers,
    Tests,
}

impl JunitSummaryGroup {
    fn from(value: &str) -> Option<Self> {
        match value {
            "containers" => Some(Self::Containers),
            "tests" => Some(Self::Tests),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junit6_command_uses_execute_classpath_scan_reports_and_no_tests_failure() {
        let classpath = [
            ".wisteria/work/test-bin",
            ".wisteria/work/bin",
            ".wisteria/cache/junit.jar",
        ]
        .join(&consts::java_seperator().to_string());
        let args = junit6_command_args(
            ".wisteria/cache/junit.jar",
            &classpath,
            "target/test-results/main/",
        );

        assert_eq!(args[0], "-jar");
        assert_eq!(args[1], ".wisteria/cache/junit.jar");
        assert!(args.contains(&String::from("execute")));
        assert!(
            args.windows(2)
                .any(|window| window == ["--class-path", classpath.as_str()])
        );
        assert!(
            args.windows(2)
                .any(|window| window == ["--scan-classpath", consts::TEST_BINARY_OUT_PATH])
        );
        assert!(
            args.windows(2)
                .any(|window| window == ["--reports-dir", "target/test-results/main/"])
        );
        assert!(args.contains(&String::from("--fail-if-no-tests")));
    }

    #[test]
    fn source_message_pluralizes_source_files() {
        assert_eq!(source_message(1, "test source"), "1 test source file");
        assert_eq!(source_message(2, "test source"), "2 test source files");
    }

    #[test]
    fn formats_junit_summary_as_horizontal_lines() {
        let output = format_junit_console_output(
            r#"
╷
├─ JUnit Jupiter ✔
│  └─ ExampleTest ✔
│     └─ passingTest() ✔
╵

Test run finished after 42 ms
[         3 containers found      ]
[         0 containers skipped    ]
[         3 containers started    ]
[         0 containers aborted    ]
[         3 containers successful ]
[         0 containers failed     ]
[         1 tests found           ]
[         0 tests skipped         ]
[         1 tests started         ]
[         0 tests aborted         ]
[         1 tests successful      ]
[         0 tests failed          ]
"#,
        );

        assert!(output.contains("├─ JUnit Jupiter ✔"));
        assert!(output.contains("Test run finished after 42 ms"));
        assert!(output.contains(
            "Containers: 3 found | 0 skipped | 3 started | 0 aborted | 3 successful | 0 failed"
        ));
        assert!(output.contains(
            "Tests: 1 found | 0 skipped | 1 started | 0 aborted | 1 successful | 0 failed"
        ));
        assert!(!output.contains("[         3 containers found"));
        assert!(!output.contains("[         1 tests found"));
    }

    #[test]
    fn leaves_non_junit_summary_output_unchanged() {
        let output = "Something unexpected\nwith multiple lines\n";

        assert_eq!(
            format_junit_console_output(output),
            "Something unexpected\nwith multiple lines"
        );
    }
}
