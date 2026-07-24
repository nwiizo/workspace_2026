pub const RUNTIME_ERROR_EXIT: i32 = 1;
pub const USAGE_ERROR_EXIT: i32 = 2;

pub fn absorb_cargo_subcommand<I>(args: I, subcommand: &str) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut cleaned = Vec::new();
    for (idx, arg) in args.into_iter().enumerate() {
        if idx == 1 && arg == subcommand {
            continue;
        }
        cleaned.push(arg);
    }
    cleaned
}

pub fn select_mode<T: Copy>(default: T, modes: &[(T, bool)]) -> T {
    modes
        .iter()
        .find_map(|(mode, enabled)| enabled.then_some(*mode))
        .unwrap_or(default)
}

pub fn warn_ignored_modes<T: Copy + Eq>(
    modes: &[(T, bool, &str)],
    selected: T,
    mode_name: impl Fn(T) -> &'static str,
) {
    let selected_name = mode_name(selected);
    for (mode, enabled, flag) in modes {
        if *enabled && mode_name(*mode) != selected_name {
            eprintln!("warning: ignoring {flag}; using {selected_name}");
        }
    }
}
