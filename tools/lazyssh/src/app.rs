use crate::config::Host;
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

pub struct App {
    hosts: Vec<Host>,
    search_texts: Vec<String>,
    visible_indices: Vec<usize>,
    matcher: Matcher,
    query: String,
    selected: usize,
    searching: bool,
    showing_help: bool,
    status: Option<String>,
}

impl App {
    #[must_use]
    pub fn new(hosts: Vec<Host>) -> Self {
        let search_texts = hosts.iter().map(search_text).collect();
        let mut app = Self {
            hosts,
            search_texts,
            visible_indices: Vec::new(),
            matcher: Matcher::new(Config::DEFAULT),
            query: String::new(),
            selected: 0,
            searching: false,
            showing_help: false,
            status: None,
        };
        app.recompute_visible_hosts();
        app
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.recompute_visible_hosts();
        self.selected = 0;
    }

    pub fn push_query(&mut self, character: char) {
        let mut query = self.query.clone();
        query.push(character);
        self.set_query(query);
    }

    pub fn pop_query(&mut self) {
        let mut query = self.query.clone();
        query.pop();
        self.set_query(query);
    }

    pub fn start_search(&mut self) {
        self.searching = true;
    }

    pub fn finish_search(&mut self) {
        self.searching = false;
    }

    pub fn cancel_search(&mut self) {
        self.searching = false;
        self.set_query(String::new());
    }

    #[must_use]
    pub fn is_searching(&self) -> bool {
        self.searching
    }

    pub fn toggle_help(&mut self) {
        self.showing_help = !self.showing_help;
    }

    #[must_use]
    pub fn is_showing_help(&self) -> bool {
        self.showing_help
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = Some(status.into());
    }

    #[must_use]
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn replace_hosts(&mut self, hosts: Vec<Host>) {
        let selected_alias = self.selected_host().map(|host| host.alias.clone());
        self.hosts = hosts;
        self.search_texts = self.hosts.iter().map(search_text).collect();
        self.recompute_visible_hosts();
        self.selected = selected_alias
            .and_then(|alias| self.position_of_alias(&alias))
            .unwrap_or(0);
    }

    #[must_use]
    pub fn visible_count(&self) -> usize {
        self.visible_indices.len()
    }

    #[must_use]
    pub fn total_count(&self) -> usize {
        self.hosts.len()
    }

    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        (!self.visible_indices.is_empty()).then_some(self.selected)
    }

    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    #[must_use]
    pub fn visible_hosts(&self) -> Vec<&Host> {
        self.visible_indices
            .iter()
            .map(|index| &self.hosts[*index])
            .collect()
    }

    #[must_use]
    pub fn selected_host(&self) -> Option<&Host> {
        self.visible_indices
            .get(self.selected)
            .map(|index| &self.hosts[*index])
    }

    pub fn select_next(&mut self) {
        let count = self.visible_indices.len();
        if count > 0 {
            self.selected = (self.selected + 1) % count;
        }
    }

    pub fn select_previous(&mut self) {
        let count = self.visible_indices.len();
        if count > 0 {
            self.selected = self.selected.checked_sub(1).unwrap_or(count - 1);
        }
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    pub fn select_last(&mut self) {
        self.selected = self.visible_indices.len().saturating_sub(1);
    }

    fn recompute_visible_hosts(&mut self) {
        if self.query.is_empty() {
            self.visible_indices = (0..self.hosts.len()).collect();
            return;
        }

        let pattern = Pattern::new(
            &self.query,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        let mut buffer = Vec::new();
        let mut matches = self
            .search_texts
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                let score =
                    pattern.score(Utf32Str::new(candidate, &mut buffer), &mut self.matcher)?;
                let alias = &self.hosts[index].alias;
                let alias_score =
                    pattern.score(Utf32Str::new(alias, &mut buffer), &mut self.matcher);
                Some((index, score, alias_score, alias.chars().count()))
            })
            .collect::<Vec<_>>();
        matches.sort_by(
            |(left_index, left_score, left_alias_score, left_alias_len),
             (right_index, right_score, right_alias_score, right_alias_len)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| right_alias_score.cmp(left_alias_score))
                    .then_with(|| left_alias_len.cmp(right_alias_len))
                    .then_with(|| left_index.cmp(right_index))
            },
        );
        self.visible_indices = matches.into_iter().map(|(index, _, _, _)| index).collect();
    }

    fn position_of_alias(&self, alias: &str) -> Option<usize> {
        self.visible_indices
            .iter()
            .position(|index| self.hosts[*index].alias == alias)
    }
}

fn search_text(host: &Host) -> String {
    [
        Some(host.alias.as_str()),
        host.host_name.as_deref(),
        host.user.as_deref(),
        host.proxy_jump.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}
