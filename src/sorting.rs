use crate::models::{InstallSource, InstalledEntry};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortColumn {
    Application,
    Publisher,
    Location,
    Source,
}

impl SortColumn {
    pub fn label(self) -> &'static str {
        match self {
            Self::Application => "APPLICATION",
            Self::Publisher => "PUBLISHER",
            Self::Location => "LOCATION",
            Self::Source => "SOURCE",
        }
    }

    fn value(self, entry: &InstalledEntry) -> &str {
        match self {
            Self::Application => &entry.display_name,
            Self::Publisher => &entry.publisher,
            Self::Location => entry.install_location.as_deref().unwrap_or(""),
            Self::Source => match entry.source {
                InstallSource::Registry => "Win32",
                InstallSource::Steam => "Steam",
                InstallSource::MicrosoftStore => "Store",
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SortState {
    pub column: SortColumn,
    pub descending: bool,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            column: SortColumn::Application,
            descending: false,
        }
    }
}

impl SortState {
    pub fn select(&mut self, column: SortColumn) {
        if self.column == column {
            self.descending = !self.descending;
        } else {
            self.column = column;
            self.descending = false;
        }
    }

    pub fn sort(&self, rows: &mut [(usize, &InstalledEntry)]) {
        // Sorting the visible references preserves the entries and selected app IDs.
        rows.sort_by_cached_key(|(_, entry)| {
            (
                self.column.value(entry).to_lowercase(),
                entry.display_name.to_lowercase(),
                entry.id.clone(),
            )
        });
        if self.descending {
            rows.reverse();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        id: &str,
        name: &str,
        publisher: &str,
        location: Option<&str>,
        source: InstallSource,
    ) -> InstalledEntry {
        InstalledEntry {
            id: id.into(),
            display_name: name.into(),
            publisher: publisher.into(),
            install_location: location.map(str::to_owned),
            uninstall_string: None,
            quiet_uninstall_string: None,
            registry_uninstall_subkey: None,
            registry_hive_is_hkcu: false,
            source,
            steam_app_id: None,
            package_full_name: None,
        }
    }

    #[test]
    fn every_column_sorts_both_directions_without_changing_entries() {
        let entries = vec![
            entry("a", "Zulu", "alpha", Some(r"C:\B"), InstallSource::Steam),
            entry("b", "bravo", "Zulu", None, InstallSource::Registry),
            entry(
                "c",
                "Alpha",
                "Bravo",
                Some(r"C:\A"),
                InstallSource::MicrosoftStore,
            ),
        ];
        for (column, expected) in [
            (SortColumn::Application, ["c", "b", "a"]),
            (SortColumn::Publisher, ["a", "c", "b"]),
            (SortColumn::Location, ["b", "c", "a"]),
            (SortColumn::Source, ["a", "c", "b"]),
        ] {
            let mut rows: Vec<_> = entries.iter().enumerate().collect();
            let mut state = SortState {
                column,
                descending: false,
            };
            state.sort(&mut rows);
            assert_eq!(
                rows.iter().map(|(_, e)| e.id.as_str()).collect::<Vec<_>>(),
                expected
            );
            state.select(column);
            state.sort(&mut rows);
            assert_eq!(
                rows.iter().map(|(_, e)| e.id.as_str()).collect::<Vec<_>>(),
                expected.into_iter().rev().collect::<Vec<_>>()
            );
        }
        assert_eq!(
            entries.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn switching_columns_starts_ascending_and_repeat_click_reverses() {
        let mut state = SortState::default();
        state.select(SortColumn::Application);
        assert!(state.descending);
        state.select(SortColumn::Publisher);
        assert!(!state.descending);
        state.select(SortColumn::Publisher);
        assert!(state.descending);
        state.select(SortColumn::Publisher);
        assert!(!state.descending);
    }
}
