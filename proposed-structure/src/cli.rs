use proposed_structure::apply_fixers;
use clap::{ArgAction, Parser};
use proposed_structure::{apply_detector_fix, detect_all, load_debian_files};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "proposed-structure",
    version,
    about = "Simple detector/fixer CLI"
)]
struct Cli {
    /// Run fixers (detect-only by default)
    #[arg(long, action = ArgAction::SetTrue)]
    fix: bool,

    /// Persist file changes (dry-run by default)
    #[arg(long, action = ArgAction::SetTrue, requires = "fix")]
    apply: bool,

    /// Restrict fix mode to specific fixer(s)
    #[arg(long = "fixer", value_name = "NAME", action = ArgAction::Append, value_delimiter = ',', requires = "fix")]
    fixers: Vec<String>,

    /// Project base path
    #[arg(long, default_value = ".")]
    path: PathBuf,
}

fn print_issues(issues: &[proposed_structure::DetectedIssue]) {
    if issues.is_empty() {
        println!("No issues found.");
        return;
    }

    println!("Detected {} issue(s):", issues.len());
    for issue in issues {
        let pkg = issue.package.as_deref().unwrap_or("<source>");
        let line = issue
            .line
            .map(|n| n.to_string())
            .unwrap_or_else(|| "?".to_string());
        let field = issue.field.as_deref().unwrap_or("<unknown>");
        println!(
            "- [{}] {} (package={pkg}, line={line}, field={field})",
            issue.tag, issue.description
        );
    }

    let mut tags: BTreeSet<&str> = BTreeSet::new();
    for issue in issues {
        tags.insert(issue.tag.as_str());
    }

    println!("\nProposed fixers:");
    for issue in issues {
        print!("{} ", issue.detector_name);
    }
}

fn selected_fixers(
    issues: &[proposed_structure::DetectedIssue],
    user_fixers: &[String],
) -> Vec<String> {
    if !user_fixers.is_empty() {
        let mut uniq = BTreeSet::new();
        for name in user_fixers {
            uniq.insert(name.clone());
        }
        return uniq.into_iter().collect();
    }

    let mut selected = BTreeSet::new();
    for issue in issues {
        selected.insert(issue.detector_name.to_string());
    }
    selected.into_iter().collect()
}


fn main() {
    let cli = Cli::parse();

    let issues = match detect_all(&cli.path) {
        Ok(issues) => issues,
        Err(err) => {
            eprintln!("Detection failed: {err}");
            std::process::exit(2);
        }
    };

    print_issues(&issues);

    if !cli.fix {
        std::process::exit(if issues.is_empty() { 0 } else { 1 });
    }

    if issues.is_empty() {
        std::process::exit(0);
    }

    let selected = selected_fixers(&issues, &cli.fixers);
    if selected.is_empty() {
        println!("No matching fixers for detected issues.");
        std::process::exit(1);
    }

    let dry_run = !cli.apply;
    println!("\nSelected fixers: {}", selected.join(", "));
    println!("Mode: {}", if dry_run { "dry-run" } else { "apply" });

    match apply_fixers(&cli.path, &issues, &selected.as_ref(), dry_run) {
        Ok(count) => {
            if dry_run {
                println!("Dry-run: {count} change(s) would be applied.");
                std::process::exit(1);
            }

            let remaining = match detect_all(&cli.path) {
                Ok(remaining) => remaining,
                Err(err) => {
                    eprintln!("Post-fix detection failed: {err}");
                    std::process::exit(2);
                }
            };
            if remaining.is_empty() {
                println!("All detected issues resolved.");
                std::process::exit(0);
            }
            println!("{} issue(s) remain after fixing.", remaining.len());
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("Fixing failed: {err}");
            std::process::exit(2);
        }
    }
}
